mod connection;
mod device;
mod error;
mod pin_server;
mod protocol;

pub use connection::{
    DeviceConnection, DiyBoardPort, SerialConnection, UsbSerialChip, find_diy_board_ports,
};
#[cfg(feature = "debug-methods")]
pub use device::DebugWalletKind;
pub use device::{CustomPinServer, Device, DeviceState, UnlockOutcome, VersionInformation};
pub use error::DeviceError;
pub use pin_server::{
    HttpPinServer, PinServer, PinServerError, PinServerPayload, PinServerRequest,
};
pub use protocol::{ErrorReply, ErrorReplyKind, Network};
