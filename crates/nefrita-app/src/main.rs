#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod board_ports;
mod device_connection;
mod device_unlock;

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
