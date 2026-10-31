#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match tauri::Builder::default().run(tauri::generate_context!()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(startup_failure) => {
            eprintln!("Nefrita could not start: {startup_failure}");
            ExitCode::FAILURE
        }
    }
}
