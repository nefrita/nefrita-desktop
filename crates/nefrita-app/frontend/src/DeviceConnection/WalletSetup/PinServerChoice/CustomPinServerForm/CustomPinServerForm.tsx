import { type SubmitEvent, useId } from "react";
import type { CustomPinServerDetails } from "../../../../bindings/CustomPinServerDetails";
import type { PinServerChoiceError } from "../../../../bindings/PinServerChoiceError";
import { isCommandError } from "../../../commandFailureMessage";

export type CustomPinServerFieldError = Extract<
  PinServerChoiceError,
  "invalid-url" | "invalid-onion-url" | "invalid-public-key" | "invalid-certificate"
>;

const fieldErrorMessages: Record<CustomPinServerFieldError, string> = {
  "invalid-url":
    "The URL has to start with http:// or https://, not end with a slash and have fewer than 120 characters.",
  "invalid-onion-url":
    "The onion URL has to follow the same rules as the URL, and its host has to end in .onion.",
  "invalid-public-key":
    "The public key has to be 33 bytes in hexadecimal, 66 characters that start with 02 or 03.",
  "invalid-certificate":
    "The certificate has to hold only PEM certificates, with fewer than 2048 characters in all.",
};

export function isCustomPinServerFieldError(
  commandFailure: unknown,
): commandFailure is CustomPinServerFieldError {
  return isCommandError(commandFailure, fieldErrorMessages);
}

function enteredText(enteredDetails: FormData, fieldName: keyof CustomPinServerDetails): string {
  const fieldEntry = enteredDetails.get(fieldName);
  return typeof fieldEntry === "string" ? fieldEntry.trim() : "";
}

function optionalEnteredText(
  enteredDetails: FormData,
  fieldName: keyof CustomPinServerDetails,
): string | null {
  const optionalText = enteredText(enteredDetails, fieldName);
  return optionalText === "" ? null : optionalText;
}

export function CustomPinServerForm({
  previousDetails,
  fieldError,
  onSend,
  onCancel,
}: {
  previousDetails: CustomPinServerDetails | null;
  fieldError: CustomPinServerFieldError | null;
  onSend: (customPinServerDetails: CustomPinServerDetails) => void;
  onCancel: () => void;
}) {
  function sendEnteredDetails(submitEvent: SubmitEvent<HTMLFormElement>) {
    submitEvent.preventDefault();
    const enteredDetails = new FormData(submitEvent.currentTarget);
    onSend({
      url: enteredText(enteredDetails, "url"),
      onionUrl: optionalEnteredText(enteredDetails, "onionUrl"),
      publicKeyHex: enteredText(enteredDetails, "publicKeyHex"),
      certificate: optionalEnteredText(enteredDetails, "certificate"),
    });
  }

  function fieldErrorMessageFor(ownFieldError: CustomPinServerFieldError): string | null {
    return fieldError === ownFieldError ? fieldErrorMessages[ownFieldError] : null;
  }

  return (
    <form onSubmit={sendEnteredDetails}>
      <DetailsField
        fieldLabel="URL"
        fieldName="url"
        previousText={previousDetails?.url ?? ""}
        fieldErrorMessage={fieldErrorMessageFor("invalid-url")}
      />
      <DetailsField
        fieldLabel="Onion URL (optional)"
        fieldName="onionUrl"
        previousText={previousDetails?.onionUrl ?? ""}
        fieldErrorMessage={fieldErrorMessageFor("invalid-onion-url")}
      />
      <DetailsField
        fieldLabel="Public key (hexadecimal)"
        fieldName="publicKeyHex"
        previousText={previousDetails?.publicKeyHex ?? ""}
        fieldErrorMessage={fieldErrorMessageFor("invalid-public-key")}
      />
      <DetailsField
        fieldLabel="Certificate (PEM, optional)"
        fieldName="certificate"
        previousText={previousDetails?.certificate ?? ""}
        fieldErrorMessage={fieldErrorMessageFor("invalid-certificate")}
        multiline
      />
      <button type="submit">Send to the board</button>
      <button type="button" onClick={onCancel}>
        Cancel
      </button>
    </form>
  );
}

function DetailsField({
  fieldLabel,
  fieldName,
  previousText,
  fieldErrorMessage,
  multiline = false,
}: {
  fieldLabel: string;
  fieldName: keyof CustomPinServerDetails;
  previousText: string;
  fieldErrorMessage: string | null;
  multiline?: boolean;
}) {
  const fieldId = useId();
  const errorMessageId = useId();
  const fieldProperties = {
    id: fieldId,
    name: fieldName,
    defaultValue: previousText,
    "aria-invalid": fieldErrorMessage !== null,
    "aria-describedby": fieldErrorMessage === null ? undefined : errorMessageId,
  };
  return (
    <p>
      <label htmlFor={fieldId}>{fieldLabel}</label>
      {multiline ? <textarea {...fieldProperties} /> : <input {...fieldProperties} />}
      {fieldErrorMessage !== null && (
        <span id={errorMessageId} role="alert">
          {fieldErrorMessage}
        </span>
      )}
    </p>
  );
}
