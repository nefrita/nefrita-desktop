import type { Channel } from "@tauri-apps/api/core";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { ConnectedDeviceState } from "../../../bindings/ConnectedDeviceState";
import type { DeviceUnlockError } from "../../../bindings/DeviceUnlockError";
import type { UnlockAttempt } from "../../../bindings/UnlockAttempt";
import type { UnlockStep } from "../../../bindings/UnlockStep";
import { DeviceUnlock } from "../DeviceUnlock";

type UnlockDeviceArguments = { unlockStepChannel: Channel<UnlockStep> };

const temporaryWalletNotice =
  "This wallet is temporary: the board erases its recovery phrase when it is powered off or when you log out.";

const eraseConfirmation =
  "Logging out erases this temporary wallet. To use it again, you have to enter its recovery phrase on the board.";

const unlockedSavedWallet: UnlockAttempt = { outcome: "unlocked", deviceState: "ready" };

const unlockedTemporaryWallet: UnlockAttempt = { outcome: "unlocked", deviceState: "temporary" };

const deviceStatesWithoutASavedWallet: ConnectedDeviceState[] = ["uninitialized", "unsaved"];

const messagesByStateAfterRefusal: [ConnectedDeviceState, string][] = [
  [
    "locked",
    "The board refused the PIN. It shows how many attempts are left when you unlock it again.",
  ],
  ["uninitialized", "The board refused the PIN three times in a row and erased its wallet."],
];

const messagesByUnlockError: [DeviceUnlockError, string][] = [
  ["no-connected-device", "The app is no longer connected to the board. Connect it again."],
  ["connection-lost", "The connection to the board was lost. Check the cable and try again."],
  [
    "device-not-answering",
    "The board stopped answering. Unplug it, plug it in again and connect it.",
  ],
  ["pin-entry-cancelled", "The PIN entry was cancelled on the board."],
  [
    "network-mismatch",
    "The board is set to another network than the app, which unlocks on testnet. Change the network in the board's settings.",
  ],
  ["temporary-wallet-in-use", "The temporary wallet of the board is in use by another connection."],
  [
    "pin-server-exchange-failed",
    "The exchange with the PIN server failed. Check that the server is running and try again.",
  ],
  ["pin-server-needs-tor", "The PIN server of the board can only be reached through Tor."],
  ["pin-server-misconfigured", "The PIN server set on the board is not valid."],
  [
    "unsupported-pin-server-request",
    "The board asked for a PIN server request that the app does not support.",
  ],
  ["device-rejected-request", "The board could not complete the request."],
  [
    "device-not-recognized",
    "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.",
  ],
  ["command-interrupted", "The app stopped before it finished talking to the board."],
];

function mockUnlockCommands({
  answerUnlock = () => Promise.resolve(unlockedSavedWallet),
  answerLogout = () => Promise.resolve<ConnectedDeviceState>("locked"),
}: {
  answerUnlock?: (unlockStepChannel: Channel<UnlockStep>) => Promise<UnlockAttempt>;
  answerLogout?: () => Promise<ConnectedDeviceState>;
} = {}) {
  mockIPC((commandName, commandArguments) => {
    switch (commandName) {
      case "unlock_device":
        return answerUnlock((commandArguments as UnlockDeviceArguments).unlockStepChannel);
      case "logout_device":
        return answerLogout();
      default:
        return Promise.reject(`unexpected command ${commandName}`);
    }
  });
}

function renderDeviceUnlock({
  deviceState = "locked",
  onDeviceStateChange = () => undefined,
  onUnlockingChange = () => undefined,
}: {
  deviceState?: ConnectedDeviceState;
  onDeviceStateChange?: (deviceState: ConnectedDeviceState) => void;
  onUnlockingChange?: (unlockInProgress: boolean) => void;
} = {}) {
  render(
    <DeviceUnlock
      deviceState={deviceState}
      onDeviceStateChange={onDeviceStateChange}
      onUnlockingChange={onUnlockingChange}
    />,
  );
}

function pressUnlock() {
  fireEvent.click(screen.getByRole("button", { name: "Unlock" }));
}

async function unlockAndAskToLogOut() {
  pressUnlock();
  fireEvent.click(await screen.findByRole("button", { name: "Log out" }));
}

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

test("asks for the PIN, shows each unlock step and reports the new state", async () => {
  let reportUnlockStep: (unlockStep: UnlockStep) => void = () => undefined;
  let finishUnlock: (unlockAttempt: UnlockAttempt) => void = () => undefined;
  mockUnlockCommands({
    answerUnlock: (unlockStepChannel) => {
      reportUnlockStep = unlockStepChannel.onmessage;
      return new Promise<UnlockAttempt>((resolveUnlock) => {
        finishUnlock = resolveUnlock;
      });
    },
  });
  const onDeviceStateChange = vi.fn();
  const onUnlockingChange = vi.fn();
  renderDeviceUnlock({ onDeviceStateChange, onUnlockingChange });
  pressUnlock();
  expect(await screen.findByText("Enter your PIN on the board.")).toBeTruthy();
  act(() => reportUnlockStep("pin-server-exchange"));
  expect(screen.getByText("Contacting the PIN server…")).toBeTruthy();
  act(() => reportUnlockStep("pin-server-reply-check"));
  expect(screen.getByText("The board is checking the answer of the PIN server…")).toBeTruthy();
  finishUnlock(unlockedSavedWallet);
  await waitFor(() => expect(onDeviceStateChange).toHaveBeenCalledWith("ready"));
  expect(onUnlockingChange.mock.calls).toEqual([[true], [false]]);
});

test.each(messagesByStateAfterRefusal)(
  "explains a refused PIN that leaves the board %s",
  async (stateAfterRefusal, expectedMessage) => {
    const refusedAttempt: UnlockAttempt = { outcome: "refused", deviceState: stateAfterRefusal };
    const onDeviceStateChange = vi.fn();
    mockUnlockCommands({ answerUnlock: () => Promise.resolve(refusedAttempt) });
    renderDeviceUnlock({ onDeviceStateChange });
    pressUnlock();
    const failureParagraph = await screen.findByRole("alert");
    expect(failureParagraph.textContent).toBe(expectedMessage);
    expect(onDeviceStateChange).toHaveBeenCalledWith(stateAfterRefusal);
  },
);

test("asks to press continue while the board shows the refused PIN", async () => {
  let reportUnlockStep: (unlockStep: UnlockStep) => void = () => undefined;
  let finishUnlock: (unlockAttempt: UnlockAttempt) => void = () => undefined;
  mockUnlockCommands({
    answerUnlock: (unlockStepChannel) => {
      reportUnlockStep = unlockStepChannel.onmessage;
      return new Promise<UnlockAttempt>((resolveUnlock) => {
        finishUnlock = resolveUnlock;
      });
    },
  });
  renderDeviceUnlock();
  pressUnlock();
  await screen.findByText("Enter your PIN on the board.");
  act(() => reportUnlockStep("refused-pin"));
  expect(screen.getByText("The board refused the PIN. Press continue on the board.")).toBeTruthy();
  finishUnlock({ outcome: "refused", deviceState: "locked" });
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe(
    "The board refused the PIN. It shows how many attempts are left when you unlock it again.",
  );
});

test.each(messagesByUnlockError)(
  "explains the %s failure and offers to unlock again",
  async (unlockError, expectedMessage) => {
    mockUnlockCommands({ answerUnlock: () => Promise.reject(unlockError) });
    renderDeviceUnlock();
    pressUnlock();
    const failureParagraph = await screen.findByRole("alert");
    expect(failureParagraph.textContent).toBe(expectedMessage);
    expect(screen.getByRole("button", { name: "Unlock" })).toBeTruthy();
  },
);

test("shows a general message and logs the cause for an unknown unlock failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockUnlockCommands({
    answerUnlock: () => Promise.reject("Command unlock_device not allowed by ACL"),
  });
  renderDeviceUnlock();
  pressUnlock();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board could not be unlocked.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board could not be unlocked.",
    "Command unlock_device not allowed by ACL",
  );
});

test("unlocks a temporary wallet without asking for the PIN and offers to log out", async () => {
  mockUnlockCommands({ answerUnlock: () => Promise.resolve(unlockedTemporaryWallet) });
  renderDeviceUnlock({ deviceState: "temporary" });
  expect(screen.getByText(temporaryWalletNotice)).toBeTruthy();
  pressUnlock();
  expect(screen.getByText("Unlocking…")).toBeTruthy();
  expect(await screen.findByRole("button", { name: "Log out" })).toBeTruthy();
});

test("erases the temporary wallet only after the erase is confirmed", async () => {
  const answerLogout = vi.fn(() => Promise.resolve<ConnectedDeviceState>("uninitialized"));
  mockUnlockCommands({
    answerUnlock: () => Promise.resolve(unlockedTemporaryWallet),
    answerLogout,
  });
  const onDeviceStateChange = vi.fn();
  renderDeviceUnlock({ deviceState: "temporary", onDeviceStateChange });
  await unlockAndAskToLogOut();
  expect(screen.getByText(eraseConfirmation)).toBeTruthy();
  expect(answerLogout).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Erase the wallet" }));
  await waitFor(() => expect(onDeviceStateChange).toHaveBeenLastCalledWith("uninitialized"));
  expect(answerLogout).toHaveBeenCalledOnce();
});

test("keeps the temporary wallet when the erase is not confirmed", async () => {
  const answerLogout = vi.fn(() => Promise.resolve<ConnectedDeviceState>("uninitialized"));
  mockUnlockCommands({
    answerUnlock: () => Promise.resolve(unlockedTemporaryWallet),
    answerLogout,
  });
  renderDeviceUnlock({ deviceState: "temporary" });
  await unlockAndAskToLogOut();
  fireEvent.click(screen.getByRole("button", { name: "Keep the wallet" }));
  expect(screen.getByRole("button", { name: "Log out" })).toBeTruthy();
  expect(screen.queryByText(eraseConfirmation)).toBeNull();
  expect(answerLogout).not.toHaveBeenCalled();
});

test("locks an unlocked saved wallet without asking to confirm", async () => {
  const onDeviceStateChange = vi.fn();
  mockUnlockCommands();
  renderDeviceUnlock({ deviceState: "ready", onDeviceStateChange });
  fireEvent.click(screen.getByRole("button", { name: "Log out" }));
  await waitFor(() => expect(onDeviceStateChange).toHaveBeenCalledWith("locked"));
  expect(screen.queryByText(eraseConfirmation)).toBeNull();
});

test("shows a general message and logs the cause for an unknown logout failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockUnlockCommands({
    answerLogout: () => Promise.reject("Command logout_device not allowed by ACL"),
  });
  renderDeviceUnlock({ deviceState: "ready" });
  fireEvent.click(screen.getByRole("button", { name: "Log out" }));
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board could not be logged out.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board could not be logged out.",
    "Command logout_device not allowed by ACL",
  );
});

test.each(deviceStatesWithoutASavedWallet)(
  "explains that a %s board has no saved wallet and offers no button",
  (deviceState) => {
    mockUnlockCommands();
    renderDeviceUnlock({ deviceState });
    expect(screen.getByText("This board has no saved wallet yet.")).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
  },
);
