import { useState } from "react";
import type { ConnectedDeviceState } from "../../bindings/ConnectedDeviceState";
import type { DeviceUnlockError } from "../../bindings/DeviceUnlockError";
import type { UnlockAttempt } from "../../bindings/UnlockAttempt";
import type { UnlockStep } from "../../bindings/UnlockStep";
import { logoutDevice, unlockDevice } from "../../commands/deviceUnlock";

type WalletSessionProgress =
  | { stage: "idle"; failureMessage: string | null }
  | { stage: "unlocking"; stepText: string }
  | { stage: "confirming-logout" }
  | { stage: "logging-out" };

const idleWithoutFailure: WalletSessionProgress = { stage: "idle", failureMessage: null };

const unlockedDeviceStates: ConnectedDeviceState[] = [
  "ready",
  "temporary-unlocked",
  "unlocked-possibly-temporary",
];

const pinEntryText = "Enter your PIN on the board.";

const temporaryWalletUnlockText = "Unlocking…";

const unlockStepTexts: Record<UnlockStep, string> = {
  "pin-server-exchange": "Contacting the PIN server…",
  "pin-server-reply-check": "The board is checking the answer of the PIN server…",
  "refused-pin": "The board refused the PIN. Press continue on the board.",
};

const pinRefusedMessage =
  "The board refused the PIN. It shows how many attempts are left when you unlock it again.";

const walletErasedMessage = "The board refused the PIN three times in a row and erased its wallet.";

const temporaryWalletNotice =
  "This wallet is temporary: the board erases its recovery phrase when it is powered off or when you log out.";

const eraseConfirmation =
  "Logging out erases this temporary wallet. To use it again, you have to enter its recovery phrase on the board.";

const possiblyTemporaryWalletLogoutConfirmation =
  "If this wallet is temporary, logging out erases it. To use it again, you have to enter its recovery phrase on the board.";

const noSavedWalletMessage = "This board has no saved wallet yet.";

const unlockErrorMessages: Record<DeviceUnlockError, string> = {
  "no-connected-device": "The app is no longer connected to the board. Connect it again.",
  "connection-lost": "The connection to the board was lost. Check the cable and try again.",
  "device-not-answering":
    "The board did not answer in time. If it shows a menu or a message, go back to its main screen and try again.",
  "pin-entry-cancelled": "The PIN entry was cancelled on the board.",
  "network-mismatch":
    "The board is set to another network than the app, which unlocks on testnet. Change the network in the board's settings.",
  "temporary-wallet-in-use": "The temporary wallet of the board is in use by another connection.",
  "pin-server-exchange-failed":
    "The exchange with the PIN server failed. Check that the server is running and try again.",
  "pin-server-needs-tor": "The PIN server of the board can only be reached through Tor.",
  "pin-server-misconfigured": "The PIN server set on the board is not valid.",
  "unsupported-pin-server-request":
    "The board asked for a PIN server request that the app does not support.",
  "device-rejected-request": "The board could not complete the request.",
  "device-not-recognized":
    "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.",
  "command-interrupted": "The app stopped before it finished talking to the board.",
};

const unknownUnlockFailureMessage = "The board could not be unlocked.";

const unknownLogoutFailureMessage = "The board could not be logged out.";

function isDeviceUnlockError(commandFailure: unknown): commandFailure is DeviceUnlockError {
  return typeof commandFailure === "string" && Object.hasOwn(unlockErrorMessages, commandFailure);
}

function commandFailureMessage(commandFailure: unknown, unknownFailureMessage: string): string {
  if (isDeviceUnlockError(commandFailure)) {
    return unlockErrorMessages[commandFailure];
  }
  console.error(unknownFailureMessage, commandFailure);
  return unknownFailureMessage;
}

function unlockAttemptFailureMessage(unlockAttempt: UnlockAttempt): string | null {
  if (unlockAttempt.outcome === "unlocked") {
    return null;
  }
  return unlockAttempt.deviceState === "uninitialized" ? walletErasedMessage : pinRefusedMessage;
}

export function DeviceUnlock({
  deviceState,
  onDeviceStateChange,
  onUnlockingChange,
}: {
  deviceState: ConnectedDeviceState;
  onDeviceStateChange: (deviceState: ConnectedDeviceState) => void;
  onUnlockingChange: (unlockInProgress: boolean) => void;
}) {
  const [walletSessionProgress, setWalletSessionProgress] =
    useState<WalletSessionProgress>(idleWithoutFailure);
  const walletUnlocked = unlockedDeviceStates.includes(deviceState);
  const hasTemporaryWallet = deviceState === "temporary" || deviceState === "temporary-unlocked";
  const withoutASavedWallet = deviceState === "uninitialized" || deviceState === "unsaved";

  async function unlock() {
    const firstUnlockText = deviceState === "temporary" ? temporaryWalletUnlockText : pinEntryText;
    setWalletSessionProgress({ stage: "unlocking", stepText: firstUnlockText });
    onUnlockingChange(true);
    try {
      const unlockAttempt = await unlockDevice((unlockStep) =>
        setWalletSessionProgress({ stage: "unlocking", stepText: unlockStepTexts[unlockStep] }),
      );
      setWalletSessionProgress({
        stage: "idle",
        failureMessage: unlockAttemptFailureMessage(unlockAttempt),
      });
      onDeviceStateChange(unlockAttempt.deviceState);
    } catch (unlockFailure) {
      setWalletSessionProgress({
        stage: "idle",
        failureMessage: commandFailureMessage(unlockFailure, unknownUnlockFailureMessage),
      });
    } finally {
      onUnlockingChange(false);
    }
  }

  async function logOut() {
    setWalletSessionProgress({ stage: "logging-out" });
    try {
      const deviceStateAfterLogout = await logoutDevice();
      setWalletSessionProgress(idleWithoutFailure);
      onDeviceStateChange(deviceStateAfterLogout);
    } catch (logoutFailure) {
      setWalletSessionProgress({
        stage: "idle",
        failureMessage: commandFailureMessage(logoutFailure, unknownLogoutFailureMessage),
      });
    }
  }

  function askToLogOut() {
    if (deviceState === "ready") {
      logOut();
    } else {
      setWalletSessionProgress({ stage: "confirming-logout" });
    }
  }

  function cancelLogout() {
    setWalletSessionProgress(idleWithoutFailure);
  }

  function walletSessionStage() {
    switch (walletSessionProgress.stage) {
      case "idle":
        return walletUnlocked ? (
          <button type="button" onClick={askToLogOut}>
            Log out
          </button>
        ) : (
          <button type="button" onClick={unlock}>
            Unlock
          </button>
        );
      case "unlocking":
        return <p>{walletSessionProgress.stepText}</p>;
      case "confirming-logout":
        return deviceState === "temporary-unlocked" ? (
          <>
            <p>{eraseConfirmation}</p>
            <button type="button" onClick={logOut}>
              Erase the wallet
            </button>
            <button type="button" onClick={cancelLogout}>
              Keep the wallet
            </button>
          </>
        ) : (
          <>
            <p>{possiblyTemporaryWalletLogoutConfirmation}</p>
            <button type="button" onClick={logOut}>
              Log out
            </button>
            <button type="button" onClick={cancelLogout}>
              Cancel
            </button>
          </>
        );
      case "logging-out":
        return (
          <button type="button" disabled>
            Log out
          </button>
        );
    }
  }

  return (
    <>
      {walletSessionProgress.stage === "idle" && walletSessionProgress.failureMessage !== null && (
        <p role="alert">{walletSessionProgress.failureMessage}</p>
      )}
      {hasTemporaryWallet && <p>{temporaryWalletNotice}</p>}
      {withoutASavedWallet ? <p>{noSavedWalletMessage}</p> : walletSessionStage()}
    </>
  );
}
