use std::error::Error;
use std::fmt;
use std::io;

use crate::message::ErrorReply;

#[derive(Debug)]
pub enum DeviceError {
    ConnectionFailed(io::Error),
    ConnectionClosed,
    ReplyTimedOut,
    RequestEncodingFailed(String),
    MalformedMessage(String),
    UnexpectedReplyIdentifier { expected: String, received: String },
    ReplyWithoutResult,
    Rejected(ErrorReply),
    NotXbtFirmware { chain: Option<String> },
}

impl fmt::Display for DeviceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(connection_error) => {
                write!(
                    formatter,
                    "the connection to the device failed: {connection_error}"
                )
            }
            Self::ConnectionClosed => write!(formatter, "the device closed the connection"),
            Self::ReplyTimedOut => write!(formatter, "the device did not reply in time"),
            Self::RequestEncodingFailed(encoding_error_description) => {
                write!(
                    formatter,
                    "the request could not be encoded: {encoding_error_description}"
                )
            }
            Self::MalformedMessage(decoding_error_description) => {
                write!(
                    formatter,
                    "the device sent a malformed message: {decoding_error_description}"
                )
            }
            Self::UnexpectedReplyIdentifier { expected, received } => write!(
                formatter,
                "the device replied to request {received} while request {expected} was waiting"
            ),
            Self::ReplyWithoutResult => {
                write!(
                    formatter,
                    "the device reply has neither a result nor an error"
                )
            }
            Self::Rejected(error_reply) => write!(
                formatter,
                "the device rejected the request with code {}: {}",
                error_reply.code, error_reply.description
            ),
            Self::NotXbtFirmware { chain: Some(chain) } => {
                write!(
                    formatter,
                    "the device firmware is not XBT: it reports the {chain} chain"
                )
            }
            Self::NotXbtFirmware { chain: None } => {
                write!(
                    formatter,
                    "the device firmware is not XBT: it reports no chain"
                )
            }
        }
    }
}

impl Error for DeviceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConnectionFailed(connection_error) => Some(connection_error),
            _ => None,
        }
    }
}
