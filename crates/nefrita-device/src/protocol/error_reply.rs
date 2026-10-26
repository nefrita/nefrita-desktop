use serde::Deserialize;

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

#[cfg(test)]
mod tests;
