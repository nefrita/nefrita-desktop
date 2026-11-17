import { useState } from "react";
import type { CustomPinServerDetails } from "../../../bindings/CustomPinServerDetails";
import type { PinServerChoiceError } from "../../../bindings/PinServerChoiceError";
import { resetPinServer, setCustomPinServer } from "../../../commands/pinServerChoice";
import {
  commandFailureMessage,
  commandInterruptedMessage,
  connectionLostMessage,
  deviceNotRecognizedMessage,
  noConnectedDeviceMessage,
} from "../../commandFailureMessage";
import {
  type CustomPinServerFieldError,
  CustomPinServerForm,
  isCustomPinServerFieldError,
} from "./CustomPinServerForm";

export type PinServerChoiceStatus = "idle" | "editing-details" | "waiting-for-the-board";

type GeneralChoiceError = Exclude<PinServerChoiceError, CustomPinServerFieldError>;

type PinServerChoiceProgress =
  | { stage: "idle"; failureMessage: string | null }
  | { stage: "chosen"; chosenPinServerMessage: string }
  | {
      stage: "editing-details";
      previousDetails: CustomPinServerDetails | null;
      fieldError: CustomPinServerFieldError | null;
      failureMessage: string | null;
    }
  | { stage: "sending-details"; sentDetails: CustomPinServerDetails }
  | { stage: "resetting" };

const idleWithoutFailure: PinServerChoiceProgress = { stage: "idle", failureMessage: null };

const editingNewDetails: PinServerChoiceProgress = {
  stage: "editing-details",
  previousDetails: null,
  fieldError: null,
  failureMessage: null,
};

const nefritaPinServerText =
  "The board uses Nefrita's PIN server, unless someone set another one on it.";

const pinServerOnTheBoardText =
  "To check it on the board, press a button on its first screen as it powers on. In the menu that opens, go to Blind Oracle and then Settings: it shows the PIN server, or “Custom Oracle not set” when the board uses Nefrita's.";

const detailsComparisonText =
  "The app checks that your PIN server answers, and then the board shows its details. Before you confirm them on the board, check that the URL and the whole public key are the ones your PIN server printed.";

const certificateComparisonText =
  "The board also shows a hash of the certificate: check that it is the SHA-256 hash of your certificate file.";

const resetConfirmationText =
  "Confirm both questions on the board to go back to Nefrita's PIN server.";

const customPinServerChosenMessage = "The board will use your PIN server.";

const nefritaPinServerChosenMessage = "The board uses Nefrita's PIN server again.";

const generalErrorMessages: Record<GeneralChoiceError, string> = {
  "no-connected-device": noConnectedDeviceMessage,
  "connection-lost": connectionLostMessage,
  "declined-on-board": "The change was declined on the board, so nothing changed.",
  "pin-server-unreachable":
    "The app could not reach the PIN server. Check its URL, that it is running and, if you sent a certificate, that it is the one of the server.",
  "pin-server-needs-tor":
    "This PIN server can only be reached through Tor, and the app does not use Tor yet.",
  "device-rejected-details": "The board did not accept the change.",
  "device-not-recognized": deviceNotRecognizedMessage,
  "command-interrupted": commandInterruptedMessage,
};

const unknownSendFailureMessage = "The board could not take your PIN server.";

const unknownResetFailureMessage = "The board could not go back to Nefrita's PIN server.";

function pinServerChoiceStatus(
  pinServerChoiceProgress: PinServerChoiceProgress,
): PinServerChoiceStatus {
  switch (pinServerChoiceProgress.stage) {
    case "idle":
    case "chosen":
      return "idle";
    case "editing-details":
      return "editing-details";
    case "sending-details":
    case "resetting":
      return "waiting-for-the-board";
  }
}

function editingAfterSendFailure(
  sentDetails: CustomPinServerDetails,
  sendFailure: unknown,
): PinServerChoiceProgress {
  if (isCustomPinServerFieldError(sendFailure)) {
    return {
      stage: "editing-details",
      previousDetails: sentDetails,
      fieldError: sendFailure,
      failureMessage: null,
    };
  }
  return {
    stage: "editing-details",
    previousDetails: sentDetails,
    fieldError: null,
    failureMessage: commandFailureMessage(
      sendFailure,
      generalErrorMessages,
      unknownSendFailureMessage,
    ),
  };
}

export function PinServerChoice({
  onStatusChange,
}: {
  onStatusChange: (pinServerChoiceStatus: PinServerChoiceStatus) => void;
}) {
  const [pinServerChoiceProgress, setPinServerChoiceProgress] =
    useState<PinServerChoiceProgress>(idleWithoutFailure);

  function changeProgress(newPinServerChoiceProgress: PinServerChoiceProgress) {
    setPinServerChoiceProgress(newPinServerChoiceProgress);
    onStatusChange(pinServerChoiceStatus(newPinServerChoiceProgress));
  }

  function openDetailsForm() {
    changeProgress(editingNewDetails);
  }

  function closeDetailsForm() {
    changeProgress(idleWithoutFailure);
  }

  async function sendDetails(customPinServerDetails: CustomPinServerDetails) {
    changeProgress({ stage: "sending-details", sentDetails: customPinServerDetails });
    try {
      await setCustomPinServer(customPinServerDetails);
      changeProgress({ stage: "chosen", chosenPinServerMessage: customPinServerChosenMessage });
    } catch (sendFailure) {
      changeProgress(editingAfterSendFailure(customPinServerDetails, sendFailure));
    }
  }

  async function resetToNefritaPinServer() {
    changeProgress({ stage: "resetting" });
    try {
      await resetPinServer();
      changeProgress({ stage: "chosen", chosenPinServerMessage: nefritaPinServerChosenMessage });
    } catch (resetFailure) {
      changeProgress({
        stage: "idle",
        failureMessage: commandFailureMessage(
          resetFailure,
          generalErrorMessages,
          unknownResetFailureMessage,
        ),
      });
    }
  }

  const choiceButtons = (
    <>
      <button type="button" onClick={openDetailsForm}>
        Use my own PIN server
      </button>
      <button type="button" onClick={resetToNefritaPinServer}>
        Use Nefrita's PIN server again
      </button>
    </>
  );

  function choiceStage() {
    switch (pinServerChoiceProgress.stage) {
      case "idle":
        return (
          <>
            {pinServerChoiceProgress.failureMessage !== null && (
              <p role="alert">{pinServerChoiceProgress.failureMessage}</p>
            )}
            {choiceButtons}
          </>
        );
      case "chosen":
        return (
          <>
            <p>{pinServerChoiceProgress.chosenPinServerMessage}</p>
            {choiceButtons}
          </>
        );
      case "editing-details":
        return (
          <>
            {pinServerChoiceProgress.failureMessage !== null && (
              <p role="alert">{pinServerChoiceProgress.failureMessage}</p>
            )}
            <CustomPinServerForm
              previousDetails={pinServerChoiceProgress.previousDetails}
              fieldError={pinServerChoiceProgress.fieldError}
              onSend={sendDetails}
              onCancel={closeDetailsForm}
            />
          </>
        );
      case "sending-details":
        return (
          <>
            <p>{detailsComparisonText}</p>
            {pinServerChoiceProgress.sentDetails.certificate !== null && (
              <p>{certificateComparisonText}</p>
            )}
          </>
        );
      case "resetting":
        return <p>{resetConfirmationText}</p>;
    }
  }

  return (
    <section>
      <h3>PIN server</h3>
      <p>{nefritaPinServerText}</p>
      <p>{pinServerOnTheBoardText}</p>
      {choiceStage()}
    </section>
  );
}
