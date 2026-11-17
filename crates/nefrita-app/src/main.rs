#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod board_ports;
mod device_connection;
mod device_unlock;
mod pin_server_choice;
mod wallet_creation;

use std::process::ExitCode;

fn main() -> ExitCode {
    match tauri::Builder::default()
        .manage(device_connection::ConnectedDeviceSlot::default())
        .invoke_handler(tauri::generate_handler![
            board_ports::search_board_ports,
            device_connection::connect_device,
            device_connection::disconnect_device,
            device_unlock::unlock_device,
            device_unlock::logout_device,
            pin_server_choice::set_custom_pin_server,
            pin_server_choice::reset_pin_server,
            wallet_creation::create_wallet,
        ])
        .run(tauri::generate_context!())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(startup_failure) => {
            eprintln!("Nefrita could not start: {startup_failure}");
            ExitCode::FAILURE
        }
    }
}
