use nefrita_device::{DeviceError, ErrorReply};

pub(crate) fn rejection_with_code(rejection_code: i64) -> DeviceError {
    DeviceError::Rejected(ErrorReply {
        code: rejection_code,
        description: "the device rejected the request".to_owned(),
    })
}
