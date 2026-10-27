use serialport::UsbPortInfo;

use super::*;

const JADE_DIY_LIST_USB_IDENTIFIERS: [(u16, u16); 6] = [
    (0x10c4, 0xea60),
    (0x1a86, 0x55d4),
    (0x0403, 0x6001),
    (0x1a86, 0x7523),
    (0x303a, 0x4001),
    (0x303a, 0x1001),
];
const ARDUINO_UNO_VENDOR_IDENTIFIER: u16 = 0x2341;
const ARDUINO_UNO_PRODUCT_IDENTIFIER: u16 = 0x0043;
const CP210X_CHIP: UsbSerialChip = UsbSerialChip {
    name: "CP210x",
    vendor_identifier: 0x10c4,
    product_identifier: 0xea60,
};
const MACOS_BOARD_PORT_NAME: &str = "/dev/cu.usbserial-0001";
const MACOS_DIAL_IN_TWIN_PORT_NAME: &str = "/dev/tty.usbserial-0001";
const CP2104_PRODUCT_NAME: &str = "CP2104 USB to UART Bridge Controller";
const BOARD_SERIAL_NUMBER: &str = "01A2B3C4";

fn usb_port(port_name: &str, vendor_identifier: u16, product_identifier: u16) -> SerialPortInfo {
    SerialPortInfo {
        port_name: port_name.to_owned(),
        port_type: SerialPortType::UsbPort(UsbPortInfo {
            vid: vendor_identifier,
            pid: product_identifier,
            serial_number: None,
            manufacturer: None,
            product: None,
        }),
    }
}

fn port_without_usb(port_name: &str, port_type: SerialPortType) -> SerialPortInfo {
    SerialPortInfo {
        port_name: port_name.to_owned(),
        port_type,
    }
}

#[test]
fn every_usb_identifier_of_the_jade_diy_list_is_found() {
    let available_ports = JADE_DIY_LIST_USB_IDENTIFIERS
        .iter()
        .enumerate()
        .map(|(port_number, &(vendor_identifier, product_identifier))| {
            usb_port(
                &format!("/dev/ttyUSB{port_number}"),
                vendor_identifier,
                product_identifier,
            )
        })
        .collect();
    let found_identifiers: Vec<(u16, u16)> = select_diy_board_ports(available_ports)
        .into_iter()
        .map(|board_port| {
            (
                board_port.chip.vendor_identifier,
                board_port.chip.product_identifier,
            )
        })
        .collect();
    assert_eq!(found_identifiers, JADE_DIY_LIST_USB_IDENTIFIERS);
}

#[test]
fn usb_port_with_another_chip_is_left_out() {
    let available_ports = vec![usb_port(
        "/dev/ttyACM0",
        ARDUINO_UNO_VENDOR_IDENTIFIER,
        ARDUINO_UNO_PRODUCT_IDENTIFIER,
    )];
    assert_eq!(select_diy_board_ports(available_ports), []);
}

#[test]
fn ports_that_are_not_usb_are_left_out() {
    let available_ports = vec![
        port_without_usb("/dev/ttyS0", SerialPortType::PciPort),
        port_without_usb(
            "/dev/cu.Bluetooth-Incoming-Port",
            SerialPortType::BluetoothPort,
        ),
        port_without_usb("/dev/cu.debug-console", SerialPortType::Unknown),
    ];
    assert_eq!(select_diy_board_ports(available_ports), []);
}

#[test]
fn macos_dial_in_twin_of_a_board_port_is_left_out() {
    let available_ports = vec![
        usb_port(
            MACOS_BOARD_PORT_NAME,
            CP210X_CHIP.vendor_identifier,
            CP210X_CHIP.product_identifier,
        ),
        usb_port(
            MACOS_DIAL_IN_TWIN_PORT_NAME,
            CP210X_CHIP.vendor_identifier,
            CP210X_CHIP.product_identifier,
        ),
    ];
    let found_port_names: Vec<String> = select_diy_board_ports(available_ports)
        .into_iter()
        .map(|board_port| board_port.port_name)
        .collect();
    assert_eq!(found_port_names, [MACOS_BOARD_PORT_NAME]);
}

#[test]
fn board_port_keeps_its_name_chip_product_and_serial_number() {
    let available_ports = vec![SerialPortInfo {
        port_name: MACOS_BOARD_PORT_NAME.to_owned(),
        port_type: SerialPortType::UsbPort(UsbPortInfo {
            vid: CP210X_CHIP.vendor_identifier,
            pid: CP210X_CHIP.product_identifier,
            serial_number: Some(BOARD_SERIAL_NUMBER.to_owned()),
            manufacturer: None,
            product: Some(CP2104_PRODUCT_NAME.to_owned()),
        }),
    }];
    assert_eq!(
        select_diy_board_ports(available_ports),
        [DiyBoardPort {
            port_name: MACOS_BOARD_PORT_NAME.to_owned(),
            chip: CP210X_CHIP,
            product_name: Some(CP2104_PRODUCT_NAME.to_owned()),
            serial_number: Some(BOARD_SERIAL_NUMBER.to_owned()),
        }]
    );
}
