use tauri_build::{AppManifest, Attributes};

const COMMAND_NAMES: &[&str] = &[
    "search_board_ports",
    "connect_device",
    "disconnect_device",
    "unlock_device",
    "logout_device",
];

fn main() {
    let build_attributes =
        Attributes::new().app_manifest(AppManifest::new().commands(COMMAND_NAMES));
    if let Err(build_failure) = tauri_build::try_build(build_attributes) {
        panic!("the Tauri build step failed: {build_failure:#}");
    }
}
