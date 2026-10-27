use std::io;

use serialport::{SerialPortInfo, SerialPortType};

const MACOS_DIAL_IN_PORT_PREFIX: &str = "/dev/tty.";

const DIY_BOARD_USB_SERIAL_CHIPS: [UsbSerialChip; 6] = [
    UsbSerialChip {
        name: "CP210x",
        vendor_identifier: 0x10c4,
        product_identifier: 0xea60,
    },
    UsbSerialChip {
        name: "CH9102",
        vendor_identifier: 0x1a86,
        product_identifier: 0x55d4,
    },
    UsbSerialChip {
        name: "FT232R",
        vendor_identifier: 0x0403,
        product_identifier: 0x6001,
    },
    UsbSerialChip {
        name: "CH340",
        vendor_identifier: 0x1a86,
        product_identifier: 0x7523,
    },
    UsbSerialChip {
        name: "ESP32-S3 TinyUSB",
        vendor_identifier: 0x303a,
        product_identifier: 0x4001,
    },
    UsbSerialChip {
        name: "ESP32-S3 USB-Serial-JTAG",
        vendor_identifier: 0x303a,
        product_identifier: 0x1001,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbSerialChip {
    pub name: &'static str,
    pub vendor_identifier: u16,
    pub product_identifier: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiyBoardPort {
    pub port_name: String,
    pub chip: UsbSerialChip,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
}

pub fn find_diy_board_ports() -> io::Result<Vec<DiyBoardPort>> {
    let available_ports = serialport::available_ports()?;
    Ok(select_diy_board_ports(available_ports))
}

fn select_diy_board_ports(available_ports: Vec<SerialPortInfo>) -> Vec<DiyBoardPort> {
    available_ports
        .into_iter()
        .filter(|available_port| {
            !available_port
                .port_name
                .starts_with(MACOS_DIAL_IN_PORT_PREFIX)
        })
        .filter_map(diy_board_port)
        .collect()
}

fn diy_board_port(available_port: SerialPortInfo) -> Option<DiyBoardPort> {
    let SerialPortType::UsbPort(usb_port_information) = available_port.port_type else {
        return None;
    };
    let chip = DIY_BOARD_USB_SERIAL_CHIPS
        .into_iter()
        .find(|diy_board_chip| {
            diy_board_chip.vendor_identifier == usb_port_information.vid
                && diy_board_chip.product_identifier == usb_port_information.pid
        })?;
    Some(DiyBoardPort {
        port_name: available_port.port_name,
        chip,
        product_name: usb_port_information.product,
        serial_number: usb_port_information.serial_number,
    })
}

#[cfg(test)]
mod tests;
