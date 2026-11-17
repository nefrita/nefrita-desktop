import type { Channel } from "@tauri-apps/api/core";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { ConnectedDeviceState } from "../../../bindings/ConnectedDeviceState";
import type { UnlockStep } from "../../../bindings/UnlockStep";
import type { WalletCreation } from "../../../bindings/WalletCreation";
import type { WalletCreationOutcome } from "../../../bindings/WalletCreationOutcome";
import { WalletSetup } from "../WalletSetup";

type CreateWalletArguments = { creationStepChannel: Channel<UnlockStep> };

const noWalletText =
  "This board has no wallet yet. You create one on the board, or recover one with its recovery phrase, and then choose its PIN.";

const unsavedWalletText =
  "This board has a wallet that is not saved yet, so the board loses it when it is powered off. To save it, choose its PIN on the board.";

const creationStepsText =
  "Follow the steps on the board to create or recover the wallet, and then choose its PIN.";

const savedWalletMessage =
  "The wallet is saved, and its PIN is registered with the PIN server at http://localhost:8097.";

const walletAlreadyOnBoardMessage =
  "The board already has a wallet, so the app did not create another one.";

const unknownCreationFailureMessage = "The board could not create or save the wallet.";

const savedWallet: WalletCreation = {
  outcome: "saved",
  deviceState: "ready",
  pinServerUrl: "http://localhost:8097",
};

const walletAlreadyOnBoard: WalletCreation = {
  outcome: "wallet-already-on-board",
  deviceState: "locked",
  pinServerUrl: null,
};

const buttonsAndTextsByDeviceState: [ConnectedDeviceState, string, string][] = [
  ["uninitialized", "Create the wallet", noWalletText],
  ["unsaved", "Save the wallet", unsavedWalletText],
];

const statesAndMessagesByCancellation: [WalletCreationOutcome, ConnectedDeviceState, string][] = [
  ["creation-cancelled", "uninitialized", "The wallet creation was cancelled on the board."],
  [
    "pin-entry-cancelled",
    "unsaved",
    "The PIN entry was cancelled on the board, so the wallet is not saved yet.",
  ],
];

function mockWalletSetupCommands({
  answerCreation = () => Promise.resolve(savedWallet),
  answerCustomPinServer = () => Promise.resolve(),
}: {
  answerCreation?: (creationStepChannel: Channel<UnlockStep>) => Promise<WalletCreation>;
  answerCustomPinServer?: () => Promise<void>;
} = {}) {
  mockIPC((commandName, commandArguments) => {
    switch (commandName) {
      case "create_wallet":
        return answerCreation((commandArguments as CreateWalletArguments).creationStepChannel);
      case "set_custom_pin_server":
        return answerCustomPinServer();
      default:
        return Promise.reject(`unexpected command ${commandName}`);
    }
  });
}

function renderWalletSetup({
  deviceState = "uninitialized",
  onDeviceStateChange = () => undefined,
  onBoardWaitingChange = () => undefined,
}: {
  deviceState?: ConnectedDeviceState;
  onDeviceStateChange?: (deviceState: ConnectedDeviceState, sectionChangeMessage?: string) => void;
  onBoardWaitingChange?: (boardWaiting: boolean) => void;
} = {}) {
  render(
    <WalletSetup
      deviceState={deviceState}
      onDeviceStateChange={onDeviceStateChange}
      onBoardWaitingChange={onBoardWaitingChange}
    />,
  );
}

function pressCreateTheWallet() {
  fireEvent.click(screen.getByRole("button", { name: "Create the wallet" }));
}

function pressUseMyOwnPinServer() {
  fireEvent.click(screen.getByRole("button", { name: "Use my own PIN server" }));
}

function pressSendToTheBoard() {
  fireEvent.click(screen.getByRole("button", { name: "Send to the board" }));
}

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

test.each(buttonsAndTextsByDeviceState)(
  "explains the %s state and offers %s next to the PIN server choice",
  (deviceState, expectedButtonName, expectedText) => {
    mockWalletSetupCommands();
    renderWalletSetup({ deviceState });
    expect(screen.getByText(expectedText)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Use my own PIN server" })).toBeTruthy();
    expect(screen.getByRole("button", { name: expectedButtonName })).toBeTruthy();
  },
);

test("asks to follow the board, shows each step and reports the new state with the saved wallet and its PIN server", async () => {
  let reportCreationStep: (creationStep: UnlockStep) => void = () => undefined;
  let finishCreation: (walletCreation: WalletCreation) => void = () => undefined;
  mockWalletSetupCommands({
    answerCreation: (creationStepChannel) => {
      reportCreationStep = creationStepChannel.onmessage;
      return new Promise<WalletCreation>((resolveCreation) => {
        finishCreation = resolveCreation;
      });
    },
  });
  const onDeviceStateChange = vi.fn();
  const onBoardWaitingChange = vi.fn();
  renderWalletSetup({ onDeviceStateChange, onBoardWaitingChange });
  pressCreateTheWallet();
  expect(screen.getByText(creationStepsText)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Use my own PIN server" })).toBeNull();
  act(() => reportCreationStep("pin-server-exchange"));
  expect(screen.getByText("Contacting the PIN server…")).toBeTruthy();
  act(() => reportCreationStep("pin-server-reply-check"));
  expect(screen.getByText("The board is checking the answer of the PIN server…")).toBeTruthy();
  finishCreation(savedWallet);
  await waitFor(() => expect(onBoardWaitingChange).toHaveBeenCalledWith(false));
  expect(onDeviceStateChange).toHaveBeenCalledWith("ready", savedWalletMessage);
  expect(onBoardWaitingChange.mock.calls).toEqual([[true], [false]]);
});

test("asks only for the PIN of an unsaved wallet", () => {
  mockWalletSetupCommands({ answerCreation: () => new Promise<WalletCreation>(() => undefined) });
  renderWalletSetup({ deviceState: "unsaved" });
  fireEvent.click(screen.getByRole("button", { name: "Save the wallet" }));
  expect(screen.getByText("Choose a PIN for the wallet on the board.")).toBeTruthy();
});

test("says only that the wallet is saved when no PIN server was named", async () => {
  const onDeviceStateChange = vi.fn();
  mockWalletSetupCommands({
    answerCreation: () => Promise.resolve({ ...savedWallet, pinServerUrl: null }),
  });
  renderWalletSetup({ onDeviceStateChange });
  pressCreateTheWallet();
  await waitFor(() =>
    expect(onDeviceStateChange).toHaveBeenCalledWith("ready", "The wallet is saved."),
  );
});

test("reports the new state with a wallet that was already on the board", async () => {
  const onDeviceStateChange = vi.fn();
  mockWalletSetupCommands({ answerCreation: () => Promise.resolve(walletAlreadyOnBoard) });
  renderWalletSetup({ onDeviceStateChange });
  pressCreateTheWallet();
  await waitFor(() =>
    expect(onDeviceStateChange).toHaveBeenCalledWith("locked", walletAlreadyOnBoardMessage),
  );
});

test.each(statesAndMessagesByCancellation)(
  "explains the %s outcome and leaves the board %s",
  async (creationOutcome, deviceStateAfterCreation, expectedMessage) => {
    const cancelledCreation: WalletCreation = {
      outcome: creationOutcome,
      deviceState: deviceStateAfterCreation,
      pinServerUrl: null,
    };
    const onDeviceStateChange = vi.fn();
    mockWalletSetupCommands({ answerCreation: () => Promise.resolve(cancelledCreation) });
    renderWalletSetup({ onDeviceStateChange });
    pressCreateTheWallet();
    const failureParagraph = await screen.findByRole("alert");
    expect(failureParagraph.textContent).toBe(expectedMessage);
    expect(onDeviceStateChange).toHaveBeenCalledWith(deviceStateAfterCreation);
  },
);

test("explains a failed creation with the message of its code", async () => {
  mockWalletSetupCommands({ answerCreation: () => Promise.reject("pin-server-exchange-failed") });
  renderWalletSetup();
  pressCreateTheWallet();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe(
    "The exchange with the PIN server failed. Check that the server is running and try again.",
  );
  expect(screen.getByRole("button", { name: "Create the wallet" })).toBeTruthy();
});

test("shows a general message and logs the cause for an unknown creation failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockWalletSetupCommands({
    answerCreation: () => Promise.reject("Command create_wallet not allowed by ACL"),
  });
  renderWalletSetup();
  pressCreateTheWallet();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe(unknownCreationFailureMessage);
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    unknownCreationFailureMessage,
    "Command create_wallet not allowed by ACL",
  );
});

test("hides the wallet button while the details of a PIN server are entered", () => {
  mockWalletSetupCommands();
  renderWalletSetup();
  pressUseMyOwnPinServer();
  expect(screen.queryByRole("button", { name: "Create the wallet" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.getByRole("button", { name: "Create the wallet" })).toBeTruthy();
});

test("reports the board waiting and hides the wallet button while it confirms a PIN server", () => {
  mockWalletSetupCommands({ answerCustomPinServer: () => new Promise<void>(() => undefined) });
  const onBoardWaitingChange = vi.fn();
  renderWalletSetup({ onBoardWaitingChange });
  pressUseMyOwnPinServer();
  pressSendToTheBoard();
  expect(onBoardWaitingChange).toHaveBeenLastCalledWith(true);
  expect(screen.queryByRole("button", { name: "Create the wallet" })).toBeNull();
});

test("keeps the creation message while the details are entered and drops it once the board waits", async () => {
  const cancelledCreation: WalletCreation = {
    outcome: "creation-cancelled",
    deviceState: "uninitialized",
    pinServerUrl: null,
  };
  mockWalletSetupCommands({
    answerCreation: () => Promise.resolve(cancelledCreation),
    answerCustomPinServer: () => new Promise<void>(() => undefined),
  });
  renderWalletSetup();
  pressCreateTheWallet();
  await screen.findByRole("alert");
  pressUseMyOwnPinServer();
  expect(screen.getByRole("alert")).toBeTruthy();
  pressSendToTheBoard();
  expect(screen.queryByRole("alert")).toBeNull();
});
