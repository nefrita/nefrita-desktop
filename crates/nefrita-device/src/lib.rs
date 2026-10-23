mod device;
mod error;
mod message;
#[cfg(test)]
mod scripted_connection;

pub use device::{Device, DeviceState, VersionInformation};
pub use error::DeviceError;
pub use message::{ErrorReply, ErrorReplyKind};
