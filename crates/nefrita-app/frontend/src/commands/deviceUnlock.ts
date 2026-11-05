import { Channel, invoke } from "@tauri-apps/api/core";
import type { ConnectedDeviceState } from "../bindings/ConnectedDeviceState";
import type { UnlockAttempt } from "../bindings/UnlockAttempt";
import type { UnlockStep } from "../bindings/UnlockStep";

export function unlockDevice(
  onUnlockStep: (unlockStep: UnlockStep) => void,
): Promise<UnlockAttempt> {
  const unlockStepChannel = new Channel<UnlockStep>(onUnlockStep);
  return invoke<UnlockAttempt>("unlock_device", { unlockStepChannel });
}

export function logoutDevice(): Promise<ConnectedDeviceState> {
  return invoke<ConnectedDeviceState>("logout_device");
}
