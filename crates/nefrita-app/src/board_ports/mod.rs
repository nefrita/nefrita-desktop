use std::io;

use nefrita_device::{DiyBoardPort, find_diy_board_ports};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct BoardPort {
    port_name: String,
    chip_name: String,
    product_name: Option<String>,
    serial_number: Option<String>,
}

impl From<DiyBoardPort> for BoardPort {
    fn from(diy_board_port: DiyBoardPort) -> Self {
        Self {
            port_name: diy_board_port.port_name,
            chip_name: diy_board_port.chip.name.to_owned(),
            product_name: diy_board_port.product_name,
            serial_number: diy_board_port.serial_number,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "kebab-case")]
pub enum BoardPortSearchError {
    PortEnumerationFailed,
    SearchInterrupted,
}

#[tauri::command]
pub async fn search_board_ports() -> Result<Vec<BoardPort>, BoardPortSearchError> {
    let search_attempt = tauri::async_runtime::spawn_blocking(find_diy_board_ports)
        .await
        .map_err(|_| BoardPortSearchError::SearchInterrupted)?;
    board_ports_from_search(search_attempt)
}

fn board_ports_from_search(
    search_attempt: io::Result<Vec<DiyBoardPort>>,
) -> Result<Vec<BoardPort>, BoardPortSearchError> {
    let diy_board_ports =
        search_attempt.map_err(|_| BoardPortSearchError::PortEnumerationFailed)?;
    Ok(diy_board_ports.into_iter().map(BoardPort::from).collect())
}

#[cfg(test)]
mod tests;
