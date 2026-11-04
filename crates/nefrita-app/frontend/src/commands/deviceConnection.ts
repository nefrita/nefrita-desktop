import { invoke } from "@tauri-apps/api/core";
import type { ConnectedDevice } from "../bindings/ConnectedDevice";

export function connectDevice(portName: string): Promise<ConnectedDevice> {
  return invoke<ConnectedDevice>("connect_device", { portName });
}

export function disconnectDevice(): Promise<void> {
  return invoke<void>("disconnect_device");
}
