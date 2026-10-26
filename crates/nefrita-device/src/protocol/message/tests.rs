use serde_json::{Value, json};

use super::*;
use crate::connection::scripted::{
    ScriptedConnection, ScriptedRead, encode_cbor_message, encode_log_message, reads_all_at_once,
    reads_byte_by_byte,
};
use crate::protocol::ErrorReplyKind;

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
    let method_result =
        read_reply_from(reads_byte_by_byte(&reply_message), "1").expect("the reply should be read");
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
    let mut connection = ScriptedConnection::new(vec![ScriptedRead::Bytes(two_encoded_replies)]);
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
    let read_attempt = read_reply_from(reads_all_at_once(&json!({"id": "2", "result": true})), "1");
    let Err(DeviceError::UnexpectedReplyIdentifier { expected, received }) = read_attempt else {
        panic!("expected a reply to another request, got {read_attempt:?}");
    };
    assert_eq!((expected.as_str(), received.as_str()), ("1", "2"));
}

#[test]
fn reply_without_result_or_error_is_reported() {
    let read_attempt = read_reply_from(reads_all_at_once(&json!({"id": "1"})), "1");
    assert!(matches!(read_attempt, Err(DeviceError::ReplyWithoutResult)));
}
