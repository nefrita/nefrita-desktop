use serde_json::json;

use super::*;
use crate::device::scripted::{confirmation_reply, device_replying_with};

const PUBLIC_TEST_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn clean_reset_is_requested_without_parameters() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .debug_clean_reset()
        .expect("the clean reset should be confirmed");
    assert_eq!(
        device.connection().written_messages(),
        vec![json!({"id": "1", "method": "debug_clean_reset"})]
    );
}

#[test]
fn mnemonic_is_sent_with_its_wallet_kind() {
    let wallet_kinds_and_temporary_flags = [
        (DebugWalletKind::SavedWithPin, false),
        (DebugWalletKind::Temporary, true),
    ];
    for (wallet_kind, temporary_flag) in wallet_kinds_and_temporary_flags {
        let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
        device
            .debug_set_mnemonic(PUBLIC_TEST_MNEMONIC, wallet_kind)
            .expect("the mnemonic should be confirmed");
        assert_eq!(
            device.connection().written_messages(),
            vec![json!({
                "id": "1",
                "method": "debug_set_mnemonic",
                "params": {"mnemonic": PUBLIC_TEST_MNEMONIC, "temporary_wallet": temporary_flag},
            })],
            "wallet kind {wallet_kind:?}"
        );
    }
}

#[test]
fn pin_is_sent_as_a_number() {
    let mut device = device_replying_with(vec![confirmation_reply("1", true)]);
    device
        .debug_set_pin_to_enter(123456)
        .expect("the PIN should be confirmed");
    assert_eq!(
        device.connection().written_messages(),
        vec![json!({"id": "1", "method": "debug_set_pin", "params": {"pin": 123456}})]
    );
}
