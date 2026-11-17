import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { CustomPinServerDetails } from "../../../../bindings/CustomPinServerDetails";
import type { PinServerChoiceError } from "../../../../bindings/PinServerChoiceError";
import { PinServerChoice, type PinServerChoiceStatus } from "../PinServerChoice";

type SetCustomPinServerArguments = { customPinServerDetails: CustomPinServerDetails };

const pinServerUrl = "http://localhost:8097";

const pinServerOnionUrl = "http://pinserverexample.onion";

const publicKeyHex = `02${"ab".repeat(32)}`;

const certificate = "-----BEGIN CERTIFICATE-----\nTUlJQmNlcnRpZmljYXRl\n-----END CERTIFICATE-----";

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

const declinedOnBoardMessage = "The change was declined on the board, so nothing changed.";

const messagesByGeneralError: [PinServerChoiceError, string][] = [
  ["no-connected-device", "The app is no longer connected to the board. Connect it again."],
  ["connection-lost", "The connection to the board was lost. Check the cable and try again."],
  ["declined-on-board", declinedOnBoardMessage],
  [
    "pin-server-unreachable",
    "The app could not reach the PIN server. Check its URL, that it is running and, if you sent a certificate, that it is the one of the server.",
  ],
  [
    "pin-server-needs-tor",
    "This PIN server can only be reached through Tor, and the app does not use Tor yet.",
  ],
  ["device-rejected-details", "The board did not accept the change."],
  [
    "device-not-recognized",
    "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.",
  ],
  ["command-interrupted", "The app stopped before it finished talking to the board."],
];

function mockPinServerChoiceCommands({
  answerCustomPinServer = () => Promise.resolve(),
  answerReset = () => Promise.resolve(),
}: {
  answerCustomPinServer?: (customPinServerDetails: CustomPinServerDetails) => Promise<void>;
  answerReset?: () => Promise<void>;
} = {}) {
  mockIPC((commandName, commandArguments) => {
    switch (commandName) {
      case "set_custom_pin_server":
        return answerCustomPinServer(
          (commandArguments as SetCustomPinServerArguments).customPinServerDetails,
        );
      case "reset_pin_server":
        return answerReset();
      default:
        return Promise.reject(`unexpected command ${commandName}`);
    }
  });
}

function renderPinServerChoice({
  onStatusChange = () => undefined,
}: {
  onStatusChange?: (pinServerChoiceStatus: PinServerChoiceStatus) => void;
} = {}) {
  render(<PinServerChoice onStatusChange={onStatusChange} />);
}

function pressUseMyOwnPinServer() {
  fireEvent.click(screen.getByRole("button", { name: "Use my own PIN server" }));
}

function pressUseNefritaPinServerAgain() {
  fireEvent.click(screen.getByRole("button", { name: "Use Nefrita's PIN server again" }));
}

function pressSendToTheBoard() {
  fireEvent.click(screen.getByRole("button", { name: "Send to the board" }));
}

function enterDetail(fieldLabel: string, enteredText: string) {
  fireEvent.change(screen.getByLabelText(fieldLabel), { target: { value: enteredText } });
}

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

test("explains that the board uses Nefrita's PIN server unless another one was set on it", () => {
  mockPinServerChoiceCommands();
  renderPinServerChoice();
  expect(screen.getByText(nefritaPinServerText)).toBeTruthy();
  expect(screen.getByText(pinServerOnTheBoardText)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Use my own PIN server" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Use Nefrita's PIN server again" })).toBeTruthy();
});

test("sends the entered details and says that the board will use the PIN server", async () => {
  const answerCustomPinServer = vi.fn(() => Promise.resolve());
  mockPinServerChoiceCommands({ answerCustomPinServer });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  enterDetail("URL", pinServerUrl);
  enterDetail("Public key (hexadecimal)", publicKeyHex);
  pressSendToTheBoard();
  expect(await screen.findByText(customPinServerChosenMessage)).toBeTruthy();
  expect(answerCustomPinServer).toHaveBeenCalledWith({
    url: pinServerUrl,
    onionUrl: null,
    publicKeyHex,
    certificate: null,
  });
  expect(screen.queryByLabelText("URL")).toBeNull();
  expect(screen.getByRole("button", { name: "Use my own PIN server" })).toBeTruthy();
});

test("sends the optional details too and asks to compare them while the board waits", () => {
  const answerCustomPinServer = vi.fn(() => new Promise<void>(() => undefined));
  mockPinServerChoiceCommands({ answerCustomPinServer });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  enterDetail("URL", pinServerUrl);
  enterDetail("Onion URL (optional)", pinServerOnionUrl);
  enterDetail("Public key (hexadecimal)", publicKeyHex);
  enterDetail("Certificate (PEM, optional)", certificate);
  pressSendToTheBoard();
  expect(screen.getByText(detailsComparisonText)).toBeTruthy();
  expect(screen.getByText(certificateComparisonText)).toBeTruthy();
  expect(screen.queryByLabelText("URL")).toBeNull();
  expect(answerCustomPinServer).toHaveBeenCalledWith({
    url: pinServerUrl,
    onionUrl: pinServerOnionUrl,
    publicKeyHex,
    certificate,
  });
});

test("asks to compare only the URL and the public key when no certificate was sent", () => {
  mockPinServerChoiceCommands({ answerCustomPinServer: () => new Promise<void>(() => undefined) });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  enterDetail("URL", pinServerUrl);
  pressSendToTheBoard();
  expect(screen.getByText(detailsComparisonText)).toBeTruthy();
  expect(screen.queryByText(certificateComparisonText)).toBeNull();
});

test("marks the failed field and keeps the details that were sent", async () => {
  mockPinServerChoiceCommands({ answerCustomPinServer: () => Promise.reject("invalid-url") });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  enterDetail("URL", `${pinServerUrl}/`);
  enterDetail("Public key (hexadecimal)", publicKeyHex);
  pressSendToTheBoard();
  const urlField = await screen.findByLabelText("URL");
  expect(urlField.getAttribute("aria-invalid")).toBe("true");
  expect(screen.getByDisplayValue(`${pinServerUrl}/`)).toBeTruthy();
  expect(screen.getByDisplayValue(publicKeyHex)).toBeTruthy();
  expect(screen.getAllByRole("alert")).toHaveLength(1);
});

test.each(messagesByGeneralError)(
  "explains the %s failure above the form",
  async (choiceError, expectedMessage) => {
    mockPinServerChoiceCommands({ answerCustomPinServer: () => Promise.reject(choiceError) });
    renderPinServerChoice();
    pressUseMyOwnPinServer();
    pressSendToTheBoard();
    const failureParagraph = await screen.findByRole("alert");
    expect(failureParagraph.textContent).toBe(expectedMessage);
    expect(screen.getByRole("button", { name: "Send to the board" })).toBeTruthy();
  },
);

test("shows a general message and logs the cause for an unknown send failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockPinServerChoiceCommands({
    answerCustomPinServer: () => Promise.reject("Command set_custom_pin_server not allowed by ACL"),
  });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  pressSendToTheBoard();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board could not take your PIN server.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board could not take your PIN server.",
    "Command set_custom_pin_server not allowed by ACL",
  );
});

test("closes the form without sending anything when the details are cancelled", () => {
  const answerCustomPinServer = vi.fn(() => Promise.resolve());
  mockPinServerChoiceCommands({ answerCustomPinServer });
  renderPinServerChoice();
  pressUseMyOwnPinServer();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.queryByLabelText("URL")).toBeNull();
  expect(answerCustomPinServer).not.toHaveBeenCalled();
});

test("asks to confirm both questions on the board and says that it uses Nefrita's PIN server again", async () => {
  let finishReset: () => void = () => undefined;
  mockPinServerChoiceCommands({
    answerReset: () => {
      return new Promise<void>((resolveReset) => {
        finishReset = () => resolveReset();
      });
    },
  });
  renderPinServerChoice();
  pressUseNefritaPinServerAgain();
  expect(screen.getByText(resetConfirmationText)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Use my own PIN server" })).toBeNull();
  finishReset();
  expect(await screen.findByText(nefritaPinServerChosenMessage)).toBeTruthy();
});

test("explains a failed reset to Nefrita's PIN server and offers the choice again", async () => {
  mockPinServerChoiceCommands({ answerReset: () => Promise.reject("declined-on-board") });
  renderPinServerChoice();
  pressUseNefritaPinServerAgain();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe(declinedOnBoardMessage);
  expect(screen.getByRole("button", { name: "Use Nefrita's PIN server again" })).toBeTruthy();
});

test("shows a general message and logs the cause for an unknown reset failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockPinServerChoiceCommands({
    answerReset: () => Promise.reject("Command reset_pin_server not allowed by ACL"),
  });
  renderPinServerChoice();
  pressUseNefritaPinServerAgain();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board could not go back to Nefrita's PIN server.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board could not go back to Nefrita's PIN server.",
    "Command reset_pin_server not allowed by ACL",
  );
});

test("reports when it waits for the board and when it is done", async () => {
  const onStatusChange = vi.fn();
  mockPinServerChoiceCommands();
  renderPinServerChoice({ onStatusChange });
  pressUseMyOwnPinServer();
  pressSendToTheBoard();
  await screen.findByText(customPinServerChosenMessage);
  pressUseNefritaPinServerAgain();
  await screen.findByText(nefritaPinServerChosenMessage);
  expect(onStatusChange.mock.calls).toEqual([
    ["editing-details"],
    ["waiting-for-the-board"],
    ["idle"],
    ["waiting-for-the-board"],
    ["idle"],
  ]);
});
