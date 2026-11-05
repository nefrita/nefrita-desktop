import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { BoardPort } from "../../bindings/BoardPort";
import type { ConnectedDevice } from "../../bindings/ConnectedDevice";
import type { DeviceConnectionError } from "../../bindings/DeviceConnectionError";
import type { UnlockAttempt } from "../../bindings/UnlockAttempt";
import { portCheckIntervalMilliseconds } from "../ConnectedDeviceDetails/ConnectedDeviceDetails";
import { DeviceConnection } from "../DeviceConnection";

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

const unlockedSavedWallet: UnlockAttempt = { outcome: "unlocked", deviceState: "ready" };

const messagesByConnectionError: [DeviceConnectionError, string][] = [
  ["port-enumeration-failed", "The system could not list its serial ports."],
  ["port-not-listed", "The board is no longer in the list. Search again and choose it."],
  [
    "port-unavailable",
    "The port is busy or the board was unplugged. Close other programs that use it and try again.",
  ],
  [
    "port-access-denied",
    "The system did not let the app open the port. On Linux, your user needs the group of the port, such as dialout.",
  ],
  ["port-open-failed", "The port could not be opened."],
  ["device-not-answering", "The board did not answer. Check that it runs the Nefrita firmware."],
  [
    "device-not-recognized",
    "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.",
  ],
  ["not-xbt-firmware", "The board does not run the XBT firmware of Nefrita."],
  ["connection-lost", "The connection to the board was lost. Check the cable and try again."],
  ["command-interrupted", "The app stopped before it finished talking to the board."],
];

function mockDeviceCommands({
  answerSearch = () => Promise.resolve([cp210xBoardPort]),
  answerConnection = () => Promise.resolve(lockedDevice),
  answerDisconnection = () => Promise.resolve(null),
}: {
  answerSearch?: () => Promise<BoardPort[]>;
  answerConnection?: (connectionArguments: unknown) => Promise<ConnectedDevice>;
  answerDisconnection?: () => Promise<null>;
} = {}) {
  mockIPC((commandName, commandArguments) => {
    switch (commandName) {
      case "search_board_ports":
        return answerSearch();
      case "connect_device":
        return answerConnection(commandArguments);
      case "disconnect_device":
        return answerDisconnection();
      case "unlock_device":
        return Promise.resolve(unlockedSavedWallet);
      default:
        return Promise.reject(`unexpected command ${commandName}`);
    }
  });
}

async function chooseTheListedBoard() {
  fireEvent.click(await screen.findByRole("button", { name: `Connect ${portName}` }));
}

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

test("connects the board chosen in the list and shows its details", async () => {
  const answerConnection = vi.fn(() => Promise.resolve(lockedDevice));
  mockDeviceCommands({ answerConnection });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  expect(await screen.findByText("1.0.36")).toBeTruthy();
  expect(answerConnection).toHaveBeenCalledWith({ portName });
});

test("shows that it is connecting until the board answers", async () => {
  mockDeviceCommands({ answerConnection: () => new Promise<ConnectedDevice>(() => undefined) });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  expect(screen.getByText(`Connecting to ${portName}…`)).toBeTruthy();
});

test.each(messagesByConnectionError)(
  "explains the %s failure and shows the list again",
  async (connectionError, expectedMessage) => {
    mockDeviceCommands({ answerConnection: () => Promise.reject(connectionError) });
    render(<DeviceConnection />);
    await chooseTheListedBoard();
    const failureParagraph = await screen.findByRole("alert");
    expect(failureParagraph.textContent).toBe(expectedMessage);
    expect(await screen.findByRole("button", { name: `Connect ${portName}` })).toBeTruthy();
  },
);

test("shows a general message and logs the cause for an unknown connection failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockDeviceCommands({
    answerConnection: () => Promise.reject("Command connect_device not allowed by ACL"),
  });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The connection to the board failed.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The connection to the board failed.",
    "Command connect_device not allowed by ACL",
  );
});

test("shows a general message and logs the cause for an unknown disconnection failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockDeviceCommands({
    answerDisconnection: () => Promise.reject("Command disconnect_device not allowed by ACL"),
  });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  fireEvent.click(await screen.findByRole("button", { name: "Disconnect" }));
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board could not be disconnected.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board could not be disconnected.",
    "Command disconnect_device not allowed by ACL",
  );
});

test("disconnects the board and shows the list again", async () => {
  const answerDisconnection = vi.fn(() => Promise.resolve(null));
  mockDeviceCommands({ answerDisconnection });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  fireEvent.click(await screen.findByRole("button", { name: "Disconnect" }));
  expect(await screen.findByRole("button", { name: `Connect ${portName}` })).toBeTruthy();
  expect(answerDisconnection).toHaveBeenCalledOnce();
  expect(screen.queryByRole("alert")).toBeNull();
});

test("explains when the board could not be disconnected", async () => {
  mockDeviceCommands({ answerDisconnection: () => Promise.reject("command-interrupted") });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  fireEvent.click(await screen.findByRole("button", { name: "Disconnect" }));
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe(
    "The app stopped before it finished talking to the board.",
  );
});

test("disconnects and warns when the board is unplugged", async () => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  const answerSearch = vi.fn(() => Promise.resolve([cp210xBoardPort]));
  const answerDisconnection = vi.fn(() => Promise.resolve(null));
  mockDeviceCommands({ answerSearch, answerDisconnection });
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  await screen.findByText("1.0.36");
  answerSearch.mockImplementation(() => Promise.resolve([]));
  await act(async () => {
    await vi.advanceTimersByTimeAsync(portCheckIntervalMilliseconds);
  });
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board was unplugged.");
  expect(answerDisconnection).toHaveBeenCalledOnce();
});

test("shows the state that the board reports after unlocking", async () => {
  mockDeviceCommands();
  render(<DeviceConnection />);
  await chooseTheListedBoard();
  fireEvent.click(await screen.findByRole("button", { name: "Unlock" }));
  expect(await screen.findByText("Unlocked")).toBeTruthy();
});
