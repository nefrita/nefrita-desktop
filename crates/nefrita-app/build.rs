use tauri_build::{AppManifest, Attributes};

fn main() {
    let build_attributes =
        Attributes::new().app_manifest(AppManifest::new().commands(&["search_board_ports"]));
    if let Err(build_failure) = tauri_build::try_build(build_attributes) {
        panic!("the Tauri build step failed: {build_failure:#}");
    }
}
