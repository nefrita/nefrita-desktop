use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use nefrita_device::{
    Device, DiyBoardPort, HttpPinServer, Network, UnlockOutcome, find_diy_board_ports,
};

const REPLY_TIMEOUT: Duration = Duration::from_secs(10);
const PORT_OPTION: &str = "--port";
const NETWORK_OPTION: &str = "--network";
const DEFAULT_NETWORK: Network = Network::XbtTestnet4;

struct ProgramOptions {
    port_name: Option<String>,
    network: Network,
}

fn main() -> ExitCode {
    match unlock_over_usb() {
        Ok(()) => ExitCode::SUCCESS,
        Err(unlock_failure) => {
            eprintln!("{unlock_failure}");
            ExitCode::FAILURE
        }
    }
}

fn unlock_over_usb() -> Result<(), Box<dyn Error>> {
    let program_options = parse_program_options(env::args().skip(1))?;
    let port_name = match program_options.port_name {
        Some(port_name) => port_name,
        None => choose_board_port_name()?,
    };
    println!("Connecting to {port_name}");
    let mut device = Device::connect_serial(&port_name, REPLY_TIMEOUT)?;
    let version_information = device.require_xbt_firmware()?;
    println!(
        "Firmware {} on {}, state {:?}",
        version_information.firmware_version,
        version_information.board_type,
        version_information.device_state
    );
    println!(
        "Unlocking on {}: follow the steps on the device",
        program_options.network.protocol_name()
    );
    match device.unlock(program_options.network, &mut HttpPinServer)? {
        UnlockOutcome::Unlocked => println!("Unlocked"),
        UnlockOutcome::Refused => println!("The device refused the PIN"),
    }
    Ok(())
}

fn parse_program_options(
    mut arguments: impl Iterator<Item = String>,
) -> Result<ProgramOptions, String> {
    let mut program_options = ProgramOptions {
        port_name: None,
        network: DEFAULT_NETWORK,
    };
    while let Some(option_name) = arguments.next() {
        match option_name.as_str() {
            PORT_OPTION => {
                program_options.port_name = Some(option_value(&mut arguments, &option_name)?);
            }
            NETWORK_OPTION => {
                program_options.network =
                    parse_network(&option_value(&mut arguments, &option_name)?)?;
            }
            _ => return Err(with_usage(format!("unknown option {option_name}"))),
        }
    }
    Ok(program_options)
}

fn option_value(
    arguments: &mut impl Iterator<Item = String>,
    option_name: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| with_usage(format!("{option_name} needs a value")))
}

fn parse_network(network_name: &str) -> Result<Network, String> {
    Network::from_protocol_name(network_name)
        .ok_or_else(|| with_usage(format!("unknown network {network_name}")))
}

fn with_usage(problem_description: String) -> String {
    let network_names = Network::ALL.map(Network::protocol_name).join("|");
    format!(
        "{problem_description}\n\
         usage: unlock_over_usb [{PORT_OPTION} <port name>] [{NETWORK_OPTION} {network_names}]"
    )
}

fn choose_board_port_name() -> Result<String, Box<dyn Error>> {
    let board_ports = find_diy_board_ports()?;
    if board_ports.is_empty() {
        return Err(format!(
            "No DIY board found: connect one, or pass {PORT_OPTION} with its port name"
        )
        .into());
    }
    println!("DIY boards found:");
    for (board_index, board_port) in board_ports.iter().enumerate() {
        println!(
            "  {}. {}",
            board_index + 1,
            board_port_description(board_port)
        );
    }
    let chosen_board_number = if board_ports.len() == 1 {
        1
    } else {
        ask_board_number(board_ports.len())?
    };
    Ok(board_ports[chosen_board_number - 1].port_name.clone())
}

fn board_port_description(board_port: &DiyBoardPort) -> String {
    format!(
        "{}  {} ({:04x}:{:04x})  product {}  serial number {}",
        board_port.port_name,
        board_port.chip.name,
        board_port.chip.vendor_identifier,
        board_port.chip.product_identifier,
        board_port.product_name.as_deref().unwrap_or("unknown"),
        board_port.serial_number.as_deref().unwrap_or("unknown"),
    )
}

fn ask_board_number(board_count: usize) -> Result<usize, Box<dyn Error>> {
    print!("Choose a board (1-{board_count}): ");
    io::stdout().flush()?;
    let mut typed_board_number = String::new();
    io::stdin().read_line(&mut typed_board_number)?;
    let typed_board_number = typed_board_number.trim();
    match typed_board_number.parse::<usize>() {
        Ok(board_number) if (1..=board_count).contains(&board_number) => Ok(board_number),
        _ => Err(
            format!("{typed_board_number} is not a board number from 1 to {board_count}").into(),
        ),
    }
}
