#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod board_ports;

use std::process::ExitCode;

fn main() -> ExitCode {
    match tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![board_ports::search_board_ports])
        .run(tauri::generate_context!())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(startup_failure) => {
            eprintln!("Nefrita could not start: {startup_failure}");
            ExitCode::FAILURE
        }
    }
}
