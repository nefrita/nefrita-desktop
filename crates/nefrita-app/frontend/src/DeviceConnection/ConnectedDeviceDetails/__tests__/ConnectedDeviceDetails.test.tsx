import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { BoardPort } from "../../../bindings/BoardPort";
import type { ConnectedDevice } from "../../../bindings/ConnectedDevice";
import type { ConnectedDeviceState } from "../../../bindings/ConnectedDeviceState";
import type { UnlockAttempt } from "../../../bindings/UnlockAttempt";
import type { WalletCreation } from "../../../bindings/WalletCreation";
import { ConnectedDeviceDetails, portCheckIntervalMilliseconds } from "../ConnectedDeviceDetails";

const portName = "/dev/cu.usbserial-0001";

const cp210xBoardPort: BoardPort = {
  portName,
  chipName: "CP210x",
  productName: null,
  serialNumber: null,
};

const lockedDevice: ConnectedDevice = {
  portName,
  firmwareVersion: "1.0.36",
  boardType: "M5StickCPlus2",
  deviceState: "locked",
};

const uninitializedDevice: ConnectedDevice = { ...lockedDevice, deviceState: "uninitialized" };

const unlockedSavedWallet: UnlockAttempt = { outcome: "unlocked", deviceState: "ready" };

const refusedPinThatErasedTheWallet: UnlockAttempt = {
  outcome: "refused",
  deviceState: "uninitialized",
};

const savedWallet: WalletCreation = {
  outcome: "saved",
  deviceState: "ready",
  pinServerUrl: "http://localhost:8097",
};

const cancelledCreation: WalletCreation = {
  outcome: "creation-cancelled",
  deviceState: "uninitialized",
  pinServerUrl: null,
};

const savedWalletMessage =
  "The wallet is saved, and its PIN is registered with the PIN server at http://localhost:8097.";

const walletErasedMessage = "The board refused the PIN three times in a row and erased its wallet.";

const labelsByDeviceState: [ConnectedDeviceState, string][] = [
  ["uninitialized", "No wallet yet"],
  ["unsaved", "Wallet not saved"],
  ["locked", "Locked"],
  ["ready", "Unlocked"],
  ["temporary", "Temporary wallet"],
  ["temporary-unlocked", "Temporary wallet, unlocked"],
  ["unlocked-possibly-temporary", "Unlocked"],
];

const walletButtonsByDeviceState: [ConnectedDeviceState, string][] = [
  ["uninitialized", "Create the wallet"],
  ["unsaved", "Save the wallet"],
];

function mockBoardPortSearch(answerSearch: () => Promise<BoardPort[]>) {
  mockIPC((commandName) => {
    if (commandName !== "search_board_ports") {
      return Promise.reject(`unexpected command ${commandName}`);
    }
    return answerSearch();
  });
}

function mockBoardCommands(boardCommandAnswers: Record<string, () => Promise<unknown>>) {
  mockIPC((commandName) => {
    if (commandName === "search_board_ports") {
      return Promise.resolve([cp210xBoardPort]);
    }
    const answerBoardCommand = boardCommandAnswers[commandName];
    if (answerBoardCommand === undefined) {
      return Promise.reject(`unexpected command ${commandName}`);
    }
    return answerBoardCommand();
  });
}

function renderConnectedDeviceDetails({
  connectedDevice = lockedDevice,
  onDisconnect = () => undefined,
  onUnplugged = () => undefined,
}: {
  connectedDevice?: ConnectedDevice;
  onDisconnect?: () => void;
  onUnplugged?: () => void;
} = {}) {
  render(
    <ConnectedDeviceDetails
      connectedDevice={connectedDevice}
      onDisconnect={onDisconnect}
      onUnplugged={onUnplugged}
      onDeviceStateChange={() => undefined}
    />,
  );
}

function ConnectedDeviceDetailsWithItsState({
  firstConnectedDevice,
}: {
  firstConnectedDevice: ConnectedDevice;
}) {
  const [connectedDevice, setConnectedDevice] = useState(firstConnectedDevice);

  function changeDeviceState(deviceState: ConnectedDeviceState) {
    setConnectedDevice({ ...connectedDevice, deviceState });
  }

  return (
    <ConnectedDeviceDetails
      connectedDevice={connectedDevice}
      onDisconnect={() => undefined}
      onUnplugged={() => undefined}
      onDeviceStateChange={changeDeviceState}
    />
  );
}

async function finishPendingCommands() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

test("shows the port, firmware version and board type of the board", () => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  renderConnectedDeviceDetails();
  expect(screen.getByText(portName)).toBeTruthy();
  expect(screen.getByText("1.0.36")).toBeTruthy();
  expect(screen.getByText("M5StickCPlus2")).toBeTruthy();
});

test.each(labelsByDeviceState)("shows the %s state as %s", (deviceState, expectedLabel) => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  renderConnectedDeviceDetails({ connectedDevice: { ...lockedDevice, deviceState } });
  expect(screen.getByText(expectedLabel)).toBeTruthy();
});

test.each(walletButtonsByDeviceState)(
  "offers the wallet setup and not the unlock to a %s board",
  (deviceState, expectedButtonName) => {
    mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
    renderConnectedDeviceDetails({ connectedDevice: { ...lockedDevice, deviceState } });
    expect(screen.getByRole("button", { name: expectedButtonName })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Unlock" })).toBeNull();
  },
);

test("offers the unlock and not the wallet setup to a locked board", () => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  renderConnectedDeviceDetails();
  expect(screen.getByRole("button", { name: "Unlock" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Use my own PIN server" })).toBeNull();
});

test("asks to disconnect when the button is pressed", () => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  const onDisconnect = vi.fn();
  renderConnectedDeviceDetails({ onDisconnect });
  fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
  expect(onDisconnect).toHaveBeenCalledOnce();
});

test("reports the board as unplugged when its port leaves the list", async () => {
  mockBoardPortSearch(() => Promise.resolve([]));
  const onUnplugged = vi.fn();
  renderConnectedDeviceDetails({ onUnplugged });
  await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds);
  expect(onUnplugged).toHaveBeenCalledOnce();
});

test("keeps the board while its port is still listed", async () => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  const onUnplugged = vi.fn();
  renderConnectedDeviceDetails({ onUnplugged });
  await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds * 3);
  expect(onUnplugged).not.toHaveBeenCalled();
});

test("checks again and logs the cause when a check fails", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockBoardPortSearch(() => Promise.reject("port-enumeration-failed"));
  const onUnplugged = vi.fn();
  renderConnectedDeviceDetails({ onUnplugged });
  await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds);
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The check of the connected board failed:",
    "port-enumeration-failed",
  );
  expect(onUnplugged).not.toHaveBeenCalled();
  mockBoardPortSearch(() => Promise.resolve([]));
  await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds);
  expect(onUnplugged).toHaveBeenCalledOnce();
});

test("reports the unplugged board only once when an earlier check answers late", async () => {
  let answerFirstCheck: (boardPorts: BoardPort[]) => void = () => undefined;
  const firstCheck = new Promise<BoardPort[]>((resolveFirstCheck) => {
    answerFirstCheck = resolveFirstCheck;
  });
  const answerSearch = vi.fn<() => Promise<BoardPort[]>>();
  answerSearch.mockReturnValueOnce(firstCheck).mockResolvedValue([]);
  mockBoardPortSearch(answerSearch);
  const onUnplugged = vi.fn();
  renderConnectedDeviceDetails({ onUnplugged });
  await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds * 2);
  answerFirstCheck([]);
  await vi.advanceTimersByTimeAsync(0);
  expect(onUnplugged).toHaveBeenCalledOnce();
});

test("hides the disconnect button while the board unlocks and shows it again afterwards", async () => {
  mockBoardCommands({ unlock_device: () => Promise.resolve(unlockedSavedWallet) });
  renderConnectedDeviceDetails();
  fireEvent.click(screen.getByRole("button", { name: "Unlock" }));
  expect(screen.queryByRole("button", { name: "Disconnect" })).toBeNull();
  await finishPendingCommands();
  expect(screen.getByRole("button", { name: "Disconnect" })).toBeTruthy();
});

test("hides the disconnect button while the board creates the wallet and shows it again afterwards", async () => {
  mockBoardCommands({ create_wallet: () => Promise.resolve(cancelledCreation) });
  renderConnectedDeviceDetails({ connectedDevice: uninitializedDevice });
  fireEvent.click(screen.getByRole("button", { name: "Create the wallet" }));
  expect(screen.queryByRole("button", { name: "Disconnect" })).toBeNull();
  await finishPendingCommands();
  expect(screen.getByRole("button", { name: "Disconnect" })).toBeTruthy();
});

test("shows the saved wallet with its PIN server above the unlock", async () => {
  mockBoardCommands({ create_wallet: () => Promise.resolve(savedWallet) });
  render(<ConnectedDeviceDetailsWithItsState firstConnectedDevice={uninitializedDevice} />);
  fireEvent.click(screen.getByRole("button", { name: "Create the wallet" }));
  await finishPendingCommands();
  expect(screen.getByText(savedWalletMessage)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Log out" })).toBeTruthy();
});

test("explains the erased wallet above the wallet setup", async () => {
  mockBoardCommands({ unlock_device: () => Promise.resolve(refusedPinThatErasedTheWallet) });
  render(<ConnectedDeviceDetailsWithItsState firstConnectedDevice={lockedDevice} />);
  fireEvent.click(screen.getByRole("button", { name: "Unlock" }));
  await finishPendingCommands();
  expect(screen.getByText(walletErasedMessage)).toBeTruthy();
  expect(screen.getByRole("button", { name: "Create the wallet" })).toBeTruthy();
});

test("drops the message of the section change once the board waits again", async () => {
  mockBoardCommands({
    unlock_device: () => Promise.resolve(refusedPinThatErasedTheWallet),
    create_wallet: () => new Promise<WalletCreation>(() => undefined),
  });
  render(<ConnectedDeviceDetailsWithItsState firstConnectedDevice={lockedDevice} />);
  fireEvent.click(screen.getByRole("button", { name: "Unlock" }));
  await finishPendingCommands();
  fireEvent.click(screen.getByRole("button", { name: "Create the wallet" }));
  expect(screen.queryByText(walletErasedMessage)).toBeNull();
});

test("drops the message of the section change once the state changes again", async () => {
  mockBoardCommands({
    create_wallet: () => Promise.resolve(savedWallet),
    logout_device: () => Promise.resolve<ConnectedDeviceState>("locked"),
  });
  render(<ConnectedDeviceDetailsWithItsState firstConnectedDevice={uninitializedDevice} />);
  fireEvent.click(screen.getByRole("button", { name: "Create the wallet" }));
  await finishPendingCommands();
  fireEvent.click(screen.getByRole("button", { name: "Log out" }));
  await finishPendingCommands();
  expect(screen.queryByText(savedWalletMessage)).toBeNull();
  expect(screen.getByText("Locked")).toBeTruthy();
});
