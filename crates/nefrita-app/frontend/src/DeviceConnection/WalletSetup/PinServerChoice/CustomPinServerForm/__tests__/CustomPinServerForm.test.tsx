import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { CustomPinServerDetails } from "../../../../../bindings/CustomPinServerDetails";
import { type CustomPinServerFieldError, CustomPinServerForm } from "../CustomPinServerForm";

const pinServerUrl = "http://localhost:8097";

const pinServerOnionUrl = "http://pinserverexample.onion";

const publicKeyHex = `02${"ab".repeat(32)}`;

const certificate = "-----BEGIN CERTIFICATE-----\nTUlJQmNlcnRpZmljYXRl\n-----END CERTIFICATE-----";

const labelsAndMessagesByFieldError: [CustomPinServerFieldError, string, string][] = [
  [
    "invalid-url",
    "URL",
    "The URL has to start with http:// or https://, not end with a slash and have fewer than 120 characters.",
  ],
  [
    "invalid-onion-url",
    "Onion URL (optional)",
    "The onion URL has to follow the same rules as the URL, and its host has to end in .onion.",
  ],
  [
    "invalid-public-key",
    "Public key (hexadecimal)",
    "The public key has to be 33 bytes in hexadecimal, 66 characters that start with 02 or 03.",
  ],
  [
    "invalid-certificate",
    "Certificate (PEM, optional)",
    "The certificate has to hold only PEM certificates, with fewer than 2048 characters in all.",
  ],
];

function renderCustomPinServerForm({
  previousDetails = null,
  fieldError = null,
  onSend = () => undefined,
  onCancel = () => undefined,
}: {
  previousDetails?: CustomPinServerDetails | null;
  fieldError?: CustomPinServerFieldError | null;
  onSend?: (customPinServerDetails: CustomPinServerDetails) => void;
  onCancel?: () => void;
} = {}) {
  render(
    <CustomPinServerForm
      previousDetails={previousDetails}
      fieldError={fieldError}
      onSend={onSend}
      onCancel={onCancel}
    />,
  );
}

function enterDetail(fieldLabel: string, enteredText: string) {
  fireEvent.change(screen.getByLabelText(fieldLabel), { target: { value: enteredText } });
}

function pressSendToTheBoard() {
  fireEvent.click(screen.getByRole("button", { name: "Send to the board" }));
}

function fieldErrorMessage(detailsField: HTMLElement): string | null | undefined {
  const errorMessageId = detailsField.getAttribute("aria-describedby") ?? "";
  return document.getElementById(errorMessageId)?.textContent;
}

afterEach(() => {
  cleanup();
});

test("sends the entered details without the blank space around them", () => {
  const onSend = vi.fn();
  renderCustomPinServerForm({ onSend });
  enterDetail("URL", ` ${pinServerUrl} `);
  enterDetail("Onion URL (optional)", ` ${pinServerOnionUrl} `);
  enterDetail("Public key (hexadecimal)", ` ${publicKeyHex} `);
  enterDetail("Certificate (PEM, optional)", `\n${certificate}\n\n`);
  pressSendToTheBoard();
  expect(onSend).toHaveBeenCalledWith({
    url: pinServerUrl,
    onionUrl: pinServerOnionUrl,
    publicKeyHex,
    certificate,
  });
});

test("sends the optional details as missing when they are left blank", () => {
  const onSend = vi.fn();
  renderCustomPinServerForm({ onSend });
  enterDetail("URL", pinServerUrl);
  enterDetail("Onion URL (optional)", " ");
  enterDetail("Public key (hexadecimal)", publicKeyHex);
  pressSendToTheBoard();
  expect(onSend).toHaveBeenCalledWith({
    url: pinServerUrl,
    onionUrl: null,
    publicKeyHex,
    certificate: null,
  });
});

test.each(labelsAndMessagesByFieldError)(
  "shows the %s failure next to the %s field",
  (fieldError, fieldLabel, expectedMessage) => {
    renderCustomPinServerForm({ fieldError });
    const failedField = screen.getByLabelText(fieldLabel);
    expect(failedField.getAttribute("aria-invalid")).toBe("true");
    expect(fieldErrorMessage(failedField)).toBe(expectedMessage);
    expect(screen.getAllByRole("alert")).toHaveLength(1);
  },
);

test("marks no field when no field failed", () => {
  renderCustomPinServerForm();
  expect(screen.getByLabelText("URL").getAttribute("aria-invalid")).toBe("false");
  expect(fieldErrorMessage(screen.getByLabelText("URL"))).toBeUndefined();
  expect(screen.queryByRole("alert")).toBeNull();
});

test("fills the fields with the details that were sent before", () => {
  renderCustomPinServerForm({
    previousDetails: { url: pinServerUrl, onionUrl: null, publicKeyHex, certificate },
  });
  expect(screen.getByLabelText<HTMLInputElement>("URL").value).toBe(pinServerUrl);
  expect(screen.getByLabelText<HTMLInputElement>("Onion URL (optional)").value).toBe("");
  expect(screen.getByLabelText<HTMLInputElement>("Public key (hexadecimal)").value).toBe(
    publicKeyHex,
  );
  expect(screen.getByLabelText<HTMLTextAreaElement>("Certificate (PEM, optional)").value).toBe(
    certificate,
  );
});

test("cancels without sending the details", () => {
  const onSend = vi.fn();
  const onCancel = vi.fn();
  renderCustomPinServerForm({ onSend, onCancel });
  enterDetail("URL", pinServerUrl);
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(onCancel).toHaveBeenCalledOnce();
  expect(onSend).not.toHaveBeenCalled();
});
