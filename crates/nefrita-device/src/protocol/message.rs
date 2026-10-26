use std::io::{ErrorKind, Read};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::DeviceError;
use crate::protocol::ErrorReply;

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
mod tests;
