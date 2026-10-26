use serde::Serialize;

use crate::connection::DeviceConnection;
use crate::device::Device;
use crate::error::DeviceError;
use crate::protocol::WITHOUT_PARAMETERS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugWalletKind {
    SavedWithPin,
    Temporary,
}

#[derive(Serialize)]
struct MnemonicParameters<'mnemonic> {
    mnemonic: &'mnemonic str,
    temporary_wallet: bool,
}

#[derive(Serialize)]
struct PinParameters {
    pin: u32,
}

impl<Connection: DeviceConnection> Device<Connection> {
    pub fn debug_clean_reset(&mut self) -> Result<(), DeviceError> {
        self.send_request_requiring_confirmation("debug_clean_reset", WITHOUT_PARAMETERS)
    }

    pub fn debug_set_mnemonic(
        &mut self,
        mnemonic: &str,
        wallet_kind: DebugWalletKind,
    ) -> Result<(), DeviceError> {
        let mnemonic_parameters = MnemonicParameters {
            mnemonic,
            temporary_wallet: wallet_kind == DebugWalletKind::Temporary,
        };
        self.send_request_requiring_confirmation("debug_set_mnemonic", Some(&mnemonic_parameters))
    }

    pub fn debug_set_pin_to_enter(&mut self, pin: u32) -> Result<(), DeviceError> {
        self.send_request_requiring_confirmation("debug_set_pin", Some(&PinParameters { pin }))
    }
}

#[cfg(test)]
mod tests;
