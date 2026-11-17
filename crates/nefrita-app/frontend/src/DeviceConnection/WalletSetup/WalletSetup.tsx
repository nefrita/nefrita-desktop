import { useState } from "react";
import type { ConnectedDeviceState } from "../../bindings/ConnectedDeviceState";
import type { WalletCreation } from "../../bindings/WalletCreation";
import { createWallet } from "../../commands/walletCreation";
import { commandFailureMessage } from "../commandFailureMessage";
import { unlockErrorMessages, unlockStepTexts } from "../DeviceUnlock";
import { PinServerChoice, type PinServerChoiceStatus } from "./PinServerChoice";

type WalletCreationProgress =
  | { stage: "idle"; failureMessage: string | null }
  | { stage: "creating"; stepText: string };

const idleWithoutFailure: WalletCreationProgress = { stage: "idle", failureMessage: null };

const noWalletText =
  "This board has no wallet yet. You create one on the board, or recover one with its recovery phrase, and then choose its PIN.";

const unsavedWalletText =
  "This board has a wallet that is not saved yet, so the board loses it when it is powered off. To save it, choose its PIN on the board.";

const creationStepsText =
  "Follow the steps on the board to create or recover the wallet, and then choose its PIN.";

const newPinEntryText = "Choose a PIN for the wallet on the board.";

const creationCancelledMessage = "The wallet creation was cancelled on the board.";

const pinEntryCancelledMessage =
  "The PIN entry was cancelled on the board, so the wallet is not saved yet.";

const walletAlreadyOnBoardMessage =
  "The board already has a wallet, so the app did not create another one.";

const unknownCreationFailureMessage = "The board could not create or save the wallet.";

function savedWalletMessage(pinServerUrl: string | null): string {
  if (pinServerUrl === null) {
    return "The wallet is saved.";
  }
  return `The wallet is saved, and its PIN is registered with the PIN server at ${pinServerUrl}.`;
}

export function WalletSetup({
  deviceState,
  onDeviceStateChange,
  onBoardWaitingChange,
}: {
  deviceState: ConnectedDeviceState;
  onDeviceStateChange: (deviceState: ConnectedDeviceState, sectionChangeMessage?: string) => void;
  onBoardWaitingChange: (boardWaiting: boolean) => void;
}) {
  const [walletCreationProgress, setWalletCreationProgress] =
    useState<WalletCreationProgress>(idleWithoutFailure);
  const [pinServerChoiceStatus, setPinServerChoiceStatus] = useState<PinServerChoiceStatus>("idle");
  const walletUnsaved = deviceState === "unsaved";

  async function createOrSaveWallet() {
    const firstCreationText = walletUnsaved ? newPinEntryText : creationStepsText;
    setWalletCreationProgress({ stage: "creating", stepText: firstCreationText });
    onBoardWaitingChange(true);
    try {
      const walletCreation = await createWallet((creationStep) =>
        setWalletCreationProgress({ stage: "creating", stepText: unlockStepTexts[creationStep] }),
      );
      applyCreationOutcome(walletCreation);
    } catch (creationFailure) {
      setWalletCreationProgress({
        stage: "idle",
        failureMessage: commandFailureMessage(
          creationFailure,
          unlockErrorMessages,
          unknownCreationFailureMessage,
        ),
      });
    } finally {
      onBoardWaitingChange(false);
    }
  }

  function applyCreationOutcome(walletCreation: WalletCreation) {
    switch (walletCreation.outcome) {
      case "saved":
        setWalletCreationProgress(idleWithoutFailure);
        onDeviceStateChange(
          walletCreation.deviceState,
          savedWalletMessage(walletCreation.pinServerUrl),
        );
        break;
      case "wallet-already-on-board":
        setWalletCreationProgress(idleWithoutFailure);
        onDeviceStateChange(walletCreation.deviceState, walletAlreadyOnBoardMessage);
        break;
      case "creation-cancelled":
        setWalletCreationProgress({ stage: "idle", failureMessage: creationCancelledMessage });
        onDeviceStateChange(walletCreation.deviceState);
        break;
      case "pin-entry-cancelled":
        setWalletCreationProgress({ stage: "idle", failureMessage: pinEntryCancelledMessage });
        onDeviceStateChange(walletCreation.deviceState);
        break;
    }
  }

  function changePinServerChoiceStatus(newPinServerChoiceStatus: PinServerChoiceStatus) {
    const boardWaiting = newPinServerChoiceStatus === "waiting-for-the-board";
    if (boardWaiting) {
      setWalletCreationProgress(idleWithoutFailure);
    }
    setPinServerChoiceStatus(newPinServerChoiceStatus);
    onBoardWaitingChange(boardWaiting);
  }

  if (walletCreationProgress.stage === "creating") {
    return <p>{walletCreationProgress.stepText}</p>;
  }

  return (
    <>
      {walletCreationProgress.failureMessage !== null && (
        <p role="alert">{walletCreationProgress.failureMessage}</p>
      )}
      <p>{walletUnsaved ? unsavedWalletText : noWalletText}</p>
      <PinServerChoice onStatusChange={changePinServerChoiceStatus} />
      {pinServerChoiceStatus === "idle" && (
        <button type="button" onClick={createOrSaveWallet}>
          {walletUnsaved ? "Save the wallet" : "Create the wallet"}
        </button>
      )}
    </>
  );
}
