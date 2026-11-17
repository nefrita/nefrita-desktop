import { Channel, invoke } from "@tauri-apps/api/core";
import type { UnlockStep } from "../bindings/UnlockStep";
import type { WalletCreation } from "../bindings/WalletCreation";

export function createWallet(
  onCreationStep: (creationStep: UnlockStep) => void,
): Promise<WalletCreation> {
  const creationStepChannel = new Channel<UnlockStep>(onCreationStep);
  return invoke<WalletCreation>("create_wallet", { creationStepChannel });
}
