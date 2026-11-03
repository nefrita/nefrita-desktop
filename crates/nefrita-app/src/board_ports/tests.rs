use std::io;

use nefrita_device::UsbSerialChip;
use serde_json::json;

use super::*;

const PORT_NAME: &str = "/dev/cu.usbserial-0001";
const PRODUCT_NAME: &str = "CP2102 USB to UART Bridge Controller";
const SERIAL_NUMBER: &str = "0001";
const CP210X_CHIP: UsbSerialChip = UsbSerialChip {
    name: "CP210x",
    vendor_identifier: 0x10c4,
    product_identifier: 0xea60,
};

fn cp210x_diy_board_port(product_name: Option<&str>, serial_number: Option<&str>) -> DiyBoardPort {
    DiyBoardPort {
        port_name: PORT_NAME.to_owned(),
        chip: CP210X_CHIP,
        product_name: product_name.map(str::to_owned),
        serial_number: serial_number.map(str::to_owned),
    }
}

#[test]
fn diy_board_port_becomes_a_board_port_with_the_chip_name() {
    let board_port = BoardPort::from(cp210x_diy_board_port(
        Some(PRODUCT_NAME),
        Some(SERIAL_NUMBER),
    ));
    assert_eq!(
        board_port,
        BoardPort {
            port_name: PORT_NAME.to_owned(),
            chip_name: CP210X_CHIP.name.to_owned(),
            product_name: Some(PRODUCT_NAME.to_owned()),
            serial_number: Some(SERIAL_NUMBER.to_owned()),
        }
    );
}

#[test]
fn board_port_serializes_with_the_field_names_that_typescript_reads() {
    let board_port = BoardPort::from(cp210x_diy_board_port(
        Some(PRODUCT_NAME),
        Some(SERIAL_NUMBER),
    ));
    assert_eq!(
        serde_json::to_value(board_port).expect("a board port should serialize"),
        json!({
            "portName": PORT_NAME,
            "chipName": CP210X_CHIP.name,
            "productName": PRODUCT_NAME,
            "serialNumber": SERIAL_NUMBER,
        })
    );
}

#[test]
fn missing_product_name_and_serial_number_serialize_as_null() {
    let board_port = BoardPort::from(cp210x_diy_board_port(None, None));
    assert_eq!(
        serde_json::to_value(board_port).expect("a board port should serialize"),
        json!({
            "portName": PORT_NAME,
            "chipName": CP210X_CHIP.name,
            "productName": null,
            "serialNumber": null,
        })
    );
}

#[test]
fn successful_port_enumeration_returns_the_board_ports() {
    let search_attempt = Ok(vec![cp210x_diy_board_port(None, None)]);
    assert_eq!(
        board_ports_from_search(search_attempt),
        Ok(vec![BoardPort::from(cp210x_diy_board_port(None, None))])
    );
}

#[test]
fn failed_port_enumeration_returns_its_error_code() {
    let search_attempt = Err(io::Error::other(
        "the system could not list its serial ports",
    ));
    assert_eq!(
        board_ports_from_search(search_attempt),
        Err(BoardPortSearchError::PortEnumerationFailed)
    );
}

#[test]
fn search_errors_serialize_as_their_codes() {
    assert_eq!(
        serde_json::to_value(BoardPortSearchError::PortEnumerationFailed)
            .expect("a search error should serialize"),
        json!("port-enumeration-failed")
    );
    assert_eq!(
        serde_json::to_value(BoardPortSearchError::SearchInterrupted)
            .expect("a search error should serialize"),
        json!("search-interrupted")
    );
}
