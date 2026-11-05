import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { BoardPort } from "../../../bindings/BoardPort";
import type { ConnectedDevice } from "../../../bindings/ConnectedDevice";
import type { ConnectedDeviceState } from "../../../bindings/ConnectedDeviceState";
import type { UnlockAttempt } from "../../../bindings/UnlockAttempt";
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

const labelsByDeviceState: [ConnectedDeviceState, string][] = [
  ["uninitialized", "No wallet yet"],
  ["unsaved", "Wallet not saved"],
  ["locked", "Locked"],
  ["ready", "Unlocked"],
  ["temporary", "Temporary wallet"],
];

function mockBoardPortSearch(answerSearch: () => Promise<BoardPort[]>) {
  mockIPC((commandName) => {
    if (commandName !== "search_board_ports") {
      return Promise.reject(`unexpected command ${commandName}`);
    }
    return answerSearch();
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
  mockIPC((commandName) => {
    switch (commandName) {
      case "search_board_ports":
        return Promise.resolve([cp210xBoardPort]);
      case "unlock_device":
        return Promise.resolve<UnlockAttempt>({ outcome: "unlocked", deviceState: "ready" });
      default:
        return Promise.reject(`unexpected command ${commandName}`);
    }
  });
  renderConnectedDeviceDetails();
  fireEvent.click(screen.getByRole("button", { name: "Unlock" }));
  expect(screen.queryByRole("button", { name: "Disconnect" })).toBeNull();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
  expect(screen.getByRole("button", { name: "Disconnect" })).toBeTruthy();
});
