import { invoke } from "@tauri-apps/api/core";
import type { BoardPort } from "../bindings/BoardPort";

export function searchBoardPorts(): Promise<BoardPort[]> {
  return invoke<BoardPort[]>("search_board_ports");
}
