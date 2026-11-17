import { invoke } from "@tauri-apps/api/core";
import type { CustomPinServerDetails } from "../bindings/CustomPinServerDetails";

export function setCustomPinServer(customPinServerDetails: CustomPinServerDetails): Promise<void> {
  return invoke<void>("set_custom_pin_server", { customPinServerDetails });
}

export function resetPinServer(): Promise<void> {
  return invoke<void>("reset_pin_server");
}
