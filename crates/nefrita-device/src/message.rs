use std::io::{ErrorKind, Read};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::DeviceError;

pub(crate) const WITHOUT_PARAMETERS: Option<&()> = None;

const UNREADABLE_REQUEST_IDENTIFIER: &str = "00";
const READ_CHUNK_LENGTH: usize = 1024;

#[derive(Serialize)]
struct Request<'request, Parameters> {
    #[serde(rename = "id")]
    identifier: &'request str,
    #[serde(rename = "method")]
    method_name: &'request str,
    #[serde(rename = "params", skip_serializing_if = "Option::is_none")]
    parameters: Option<&'request Parameters>,
}

#[derive(Deserialize)]
struct MessageEnvelope {
    #[serde(rename = "id")]
    identifier: Option<String>,
}

#[derive(Deserialize)]
struct Reply<MethodResult> {
    #[serde(rename = "id")]
    identifier: String,
    #[serde(rename = "result")]
    method_result: Option<MethodResult>,
    #[serde(rename = "error")]
    error_reply: Option<ErrorReply>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ErrorReply {
    pub code: i64,
    #[serde(rename = "message", default)]
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReplyKind {
    InvalidRequest,
    UnknownMethod,
    InvalidParameters,
    InternalError,
    UserCancelled,
    ProtocolError,
    DeviceLocked,
    NetworkMismatch,
    UnrecognizedCode,
}

impl ErrorReply {
    pub fn kind(&self) -> ErrorReplyKind {
        match self.code {
            -32600 => ErrorReplyKind::InvalidRequest,
            -32601 => ErrorReplyKind::UnknownMethod,
            -32602 => ErrorReplyKind::InvalidParameters,
            -32603 => ErrorReplyKind::InternalError,
            -32000 => ErrorReplyKind::UserCancelled,
            -32001 => ErrorReplyKind::ProtocolError,
            -32002 => ErrorReplyKind::DeviceLocked,
            -32003 => ErrorReplyKind::NetworkMismatch,
            _ => ErrorReplyKind::UnrecognizedCode,
        }
    }
}

pub(crate) fn encode_request<Parameters: Serialize>(
    identifier: &str,
    method_name: &str,
    parameters: Option<&Parameters>,
) -> Result<Vec<u8>, DeviceError> {
    let request_to_encode = Request {
        identifier,
        method_name,
        parameters,
    };
    minicbor_serde::to_vec(&request_to_encode)
        .map_err(|encoding_error| DeviceError::RequestEncodingFailed(encoding_error.to_string()))
}

#[derive(Default)]
pub(crate) struct MessageReader {
    received_bytes: Vec<u8>,
}

impl MessageReader {
    pub(crate) fn read_reply<MethodResult: DeserializeOwned>(
        &mut self,
        connection: &mut impl Read,
        request_identifier: &str,
    ) -> Result<MethodResult, DeviceError> {
        loop {
            let message_bytes = self.read_message(connection)?;
            if is_reply(&message_bytes) {
                return decode_reply(&message_bytes, request_identifier);
            }
        }
    }

    fn read_message(&mut self, connection: &mut impl Read) -> Result<Vec<u8>, DeviceError> {
        loop {
            match complete_message_length(&self.received_bytes) {
                Ok(Some(message_length)) => {
                    return Ok(self.received_bytes.drain(..message_length).collect());
                }
                Ok(None) => self.receive_more_bytes(connection)?,
                Err(malformed_message) => {
                    self.received_bytes.clear();
                    return Err(malformed_message);
                }
            }
        }
    }

    fn receive_more_bytes(&mut self, connection: &mut impl Read) -> Result<(), DeviceError> {
        let mut read_chunk = [0; READ_CHUNK_LENGTH];
        loop {
            match connection.read(&mut read_chunk) {
                Ok(0) => return Err(DeviceError::ConnectionClosed),
                Ok(read_length) => {
                    self.received_bytes
                        .extend_from_slice(&read_chunk[..read_length]);
                    return Ok(());
                }
                Err(read_error) if read_error.kind() == ErrorKind::Interrupted => {}
                Err(read_error)
                    if matches!(
                        read_error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut
                    ) =>
                {
                    return Err(DeviceError::ReplyTimedOut);
                }
                Err(read_error) => return Err(DeviceError::ConnectionFailed(read_error)),
            }
        }
    }
}

fn complete_message_length(received_bytes: &[u8]) -> Result<Option<usize>, DeviceError> {
    let mut decoder = minicbor::Decoder::new(received_bytes);
    match decoder.skip() {
        Ok(()) => Ok(Some(decoder.position())),
        Err(decoding_error) if decoding_error.is_end_of_input() => Ok(None),
        Err(decoding_error) => Err(DeviceError::MalformedMessage(decoding_error.to_string())),
    }
}

fn is_reply(message_bytes: &[u8]) -> bool {
    minicbor_serde::from_slice::<MessageEnvelope>(message_bytes)
        .is_ok_and(|envelope| envelope.identifier.is_some())
}

fn decode_reply<MethodResult: DeserializeOwned>(
    message_bytes: &[u8],
    request_identifier: &str,
) -> Result<MethodResult, DeviceError> {
    let decoded_reply: Reply<MethodResult> = minicbor_serde::from_slice(message_bytes)
        .map_err(|decoding_error| DeviceError::MalformedMessage(decoding_error.to_string()))?;
    let answers_this_request = decoded_reply.identifier == request_identifier;
    if let Some(error_reply) = decoded_reply.error_reply
        && (answers_this_request || decoded_reply.identifier == UNREADABLE_REQUEST_IDENTIFIER)
    {
        return Err(DeviceError::Rejected(error_reply));
    }
    if !answers_this_request {
        return Err(DeviceError::UnexpectedReplyIdentifier {
            expected: request_identifier.to_owned(),
            received: decoded_reply.identifier,
        });
    }
    decoded_reply
        .method_result
        .ok_or(DeviceError::ReplyWithoutResult)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::scripted_connection::{
        ScriptedConnection, ScriptedRead, encode_cbor_message, encode_log_message,
        reads_all_at_once, reads_byte_by_byte,
    };

    fn read_reply_from(
        scripted_reads: Vec<ScriptedRead>,
        request_identifier: &str,
    ) -> Result<Value, DeviceError> {
        let mut connection = ScriptedConnection::new(scripted_reads);
        MessageReader::default().read_reply(&mut connection, request_identifier)
    }

    #[test]
    fn request_without_parameters_has_only_identifier_and_method() {
        let encoded_request = encode_request("7", "get_version_info", WITHOUT_PARAMETERS)
            .expect("the request should encode");
        let decoded_request: Value =
            minicbor_serde::from_slice(&encoded_request).expect("the request should decode");
        assert_eq!(
            decoded_request,
            json!({"id": "7", "method": "get_version_info"})
        );
    }

    #[test]
    fn reply_arriving_byte_by_byte_is_read_whole() {
        let reply_message = json!({"id": "1", "result": {"state": "complete"}});
        let method_result = read_reply_from(reads_byte_by_byte(&reply_message), "1")
            .expect("the reply should be read");
        assert_eq!(method_result, json!({"state": "complete"}));
    }

    #[test]
    fn log_messages_before_the_reply_are_skipped() {
        let scripted_reads = vec![
            ScriptedRead::Bytes(encode_log_message(b"I firmware log line")),
            ScriptedRead::Bytes(encode_log_message(b"W another firmware log line")),
            ScriptedRead::Bytes(encode_cbor_message(&json!({"id": "1", "result": true}))),
        ];
        let method_result = read_reply_from(scripted_reads, "1").expect("the reply should be read");
        assert_eq!(method_result, json!(true));
    }

    #[test]
    fn two_replies_in_one_read_are_read_in_order() {
        let mut two_encoded_replies = encode_cbor_message(&json!({"id": "1", "result": "first"}));
        two_encoded_replies.extend(encode_cbor_message(&json!({"id": "2", "result": "second"})));
        let mut connection =
            ScriptedConnection::new(vec![ScriptedRead::Bytes(two_encoded_replies)]);
        let mut message_reader = MessageReader::default();
        let first_method_result: String = message_reader
            .read_reply(&mut connection, "1")
            .expect("the first reply should be read");
        let second_method_result: String = message_reader
            .read_reply(&mut connection, "2")
            .expect("the second reply should be read");
        assert_eq!(
            (first_method_result.as_str(), second_method_result.as_str()),
            ("first", "second")
        );
    }

    #[test]
    fn interrupted_reads_are_retried() {
        let mut scripted_reads = vec![ScriptedRead::Interrupted];
        scripted_reads.extend(reads_all_at_once(&json!({"id": "1", "result": true})));
        let method_result = read_reply_from(scripted_reads, "1")
            .expect("the reply should be read after the interruption");
        assert_eq!(method_result, json!(true));
    }

    #[test]
    fn connection_closed_in_the_middle_of_a_message_is_reported() {
        let encoded_reply = encode_cbor_message(&json!({"id": "1", "result": "never complete"}));
        let first_half = encoded_reply[..encoded_reply.len() / 2].to_vec();
        let read_attempt = read_reply_from(vec![ScriptedRead::Bytes(first_half)], "1");
        assert!(matches!(read_attempt, Err(DeviceError::ConnectionClosed)));
    }

    #[test]
    fn read_timeout_is_reported() {
        let read_attempt = read_reply_from(vec![ScriptedRead::TimedOut], "1");
        assert!(matches!(read_attempt, Err(DeviceError::ReplyTimedOut)));
    }

    #[test]
    fn read_failure_is_reported_as_a_connection_failure() {
        let read_attempt = read_reply_from(vec![ScriptedRead::ConnectionReset], "1");
        assert!(matches!(
            read_attempt,
            Err(DeviceError::ConnectionFailed(_))
        ));
    }

    #[test]
    fn bytes_that_are_not_cbor_are_reported_as_malformed() {
        let reserved_cbor_header = vec![0x1c];
        let read_attempt = read_reply_from(vec![ScriptedRead::Bytes(reserved_cbor_header)], "1");
        assert!(matches!(
            read_attempt,
            Err(DeviceError::MalformedMessage(_))
        ));
    }

    #[test]
    fn error_reply_is_reported_as_a_rejection_with_its_kind() {
        let error_reply_message =
            json!({"id": "1", "error": {"code": -32000, "message": "User declined"}});
        let read_attempt = read_reply_from(reads_all_at_once(&error_reply_message), "1");
        let Err(DeviceError::Rejected(rejection)) = read_attempt else {
            panic!("expected a rejection, got {read_attempt:?}");
        };
        assert_eq!(rejection.kind(), ErrorReplyKind::UserCancelled);
        assert_eq!(rejection.description, "User declined");
    }

    #[test]
    fn error_reply_to_an_unreadable_request_is_reported_as_a_rejection() {
        let error_reply_message =
            json!({"id": "00", "error": {"code": -32600, "message": "Invalid RPC"}});
        let read_attempt = read_reply_from(reads_all_at_once(&error_reply_message), "1");
        let Err(DeviceError::Rejected(rejection)) = read_attempt else {
            panic!("expected a rejection, got {read_attempt:?}");
        };
        assert_eq!(rejection.kind(), ErrorReplyKind::InvalidRequest);
    }

    #[test]
    fn reply_to_another_request_is_reported() {
        let read_attempt =
            read_reply_from(reads_all_at_once(&json!({"id": "2", "result": true})), "1");
        let Err(DeviceError::UnexpectedReplyIdentifier { expected, received }) = read_attempt
        else {
            panic!("expected a reply to another request, got {read_attempt:?}");
        };
        assert_eq!((expected.as_str(), received.as_str()), ("1", "2"));
    }

    #[test]
    fn reply_without_result_or_error_is_reported() {
        let read_attempt = read_reply_from(reads_all_at_once(&json!({"id": "1"})), "1");
        assert!(matches!(read_attempt, Err(DeviceError::ReplyWithoutResult)));
    }

    #[test]
    fn error_codes_map_to_their_kinds() {
        let codes_and_kinds = [
            (-32600, ErrorReplyKind::InvalidRequest),
            (-32601, ErrorReplyKind::UnknownMethod),
            (-32602, ErrorReplyKind::InvalidParameters),
            (-32603, ErrorReplyKind::InternalError),
            (-32000, ErrorReplyKind::UserCancelled),
            (-32001, ErrorReplyKind::ProtocolError),
            (-32002, ErrorReplyKind::DeviceLocked),
            (-32003, ErrorReplyKind::NetworkMismatch),
            (-1, ErrorReplyKind::UnrecognizedCode),
        ];
        for (code, kind) in codes_and_kinds {
            let error_reply = ErrorReply {
                code,
                description: String::new(),
            };
            assert_eq!(error_reply.kind(), kind, "code {code}");
        }
    }
}
