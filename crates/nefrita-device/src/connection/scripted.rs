use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};
use std::time::Duration;

use serde_json::Value;

use crate::connection::DeviceConnection;

pub(crate) const SCRIPTED_REPLY_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) enum ScriptedRead {
    Bytes(Vec<u8>),
    Interrupted,
    TimedOut,
    ConnectionReset,
}

pub(crate) struct ScriptedConnection {
    scripted_reads: VecDeque<ScriptedRead>,
    written_bytes: Vec<u8>,
    reply_timeout: Option<Duration>,
    reply_timeout_changes: Vec<Option<Duration>>,
}

impl ScriptedConnection {
    pub(crate) fn new(scripted_reads: Vec<ScriptedRead>) -> Self {
        Self {
            scripted_reads: scripted_reads.into(),
            written_bytes: Vec::new(),
            reply_timeout: Some(SCRIPTED_REPLY_TIMEOUT),
            reply_timeout_changes: Vec::new(),
        }
    }

    pub(crate) fn encoded_written_messages(&self) -> Vec<Vec<u8>> {
        let mut decoder = minicbor::Decoder::new(&self.written_bytes);
        let mut encoded_written_messages = Vec::new();
        while decoder.position() < self.written_bytes.len() {
            let message_start = decoder.position();
            decoder
                .skip()
                .expect("the client should write complete CBOR messages");
            encoded_written_messages
                .push(self.written_bytes[message_start..decoder.position()].to_vec());
        }
        encoded_written_messages
    }

    pub(crate) fn written_messages(&self) -> Vec<Value> {
        self.encoded_written_messages()
            .iter()
            .map(|encoded_message| {
                minicbor_serde::from_slice(encoded_message).expect("written messages should decode")
            })
            .collect()
    }

    pub(crate) fn reply_timeout_changes(&self) -> &[Option<Duration>] {
        &self.reply_timeout_changes
    }
}

impl Read for ScriptedConnection {
    fn read(&mut self, read_buffer: &mut [u8]) -> io::Result<usize> {
        match self.scripted_reads.pop_front() {
            None => Ok(0),
            Some(ScriptedRead::Interrupted) => Err(io::Error::from(ErrorKind::Interrupted)),
            Some(ScriptedRead::TimedOut) => Err(io::Error::from(ErrorKind::WouldBlock)),
            Some(ScriptedRead::ConnectionReset) => Err(io::Error::from(ErrorKind::ConnectionReset)),
            Some(ScriptedRead::Bytes(scripted_bytes)) => {
                let read_length = scripted_bytes.len().min(read_buffer.len());
                read_buffer[..read_length].copy_from_slice(&scripted_bytes[..read_length]);
                if read_length < scripted_bytes.len() {
                    let unread_bytes = scripted_bytes[read_length..].to_vec();
                    self.scripted_reads
                        .push_front(ScriptedRead::Bytes(unread_bytes));
                }
                Ok(read_length)
            }
        }
    }
}

impl Write for ScriptedConnection {
    fn write(&mut self, outgoing_bytes: &[u8]) -> io::Result<usize> {
        self.written_bytes.extend_from_slice(outgoing_bytes);
        Ok(outgoing_bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl DeviceConnection for ScriptedConnection {
    fn reply_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(self.reply_timeout)
    }

    fn set_reply_timeout(&mut self, reply_timeout: Option<Duration>) -> io::Result<()> {
        self.reply_timeout = reply_timeout;
        self.reply_timeout_changes.push(reply_timeout);
        Ok(())
    }
}

pub(crate) fn encode_cbor_message(json_message: &Value) -> Vec<u8> {
    minicbor_serde::to_vec(json_message).expect("test messages should encode")
}

pub(crate) fn encode_log_message(log_line: &[u8]) -> Vec<u8> {
    let mut encoder = minicbor::Encoder::new(Vec::new());
    encoder
        .map(1)
        .and_then(|encoder| encoder.str("log"))
        .and_then(|encoder| encoder.bytes(log_line))
        .expect("test log messages should encode");
    encoder.into_writer()
}

pub(crate) fn reads_all_at_once(json_message: &Value) -> Vec<ScriptedRead> {
    vec![ScriptedRead::Bytes(encode_cbor_message(json_message))]
}

pub(crate) fn reads_byte_by_byte(json_message: &Value) -> Vec<ScriptedRead> {
    encode_cbor_message(json_message)
        .into_iter()
        .map(|message_byte| ScriptedRead::Bytes(vec![message_byte]))
        .collect()
}
