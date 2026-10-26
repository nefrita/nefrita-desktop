use super::*;

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
