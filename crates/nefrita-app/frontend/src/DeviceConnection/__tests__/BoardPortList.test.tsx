import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { BoardPort } from "../../bindings/BoardPort";
import { BoardPortList } from "../BoardPortList";

const cp210xBoardPort: BoardPort = {
  portName: "/dev/cu.usbserial-0001",
  chipName: "CP210x",
  productName: "CP2102 USB to UART Bridge Controller",
  serialNumber: "0001",
};

function mockBoardPortSearch(answerSearch: () => Promise<BoardPort[]>) {
  mockIPC((commandName) => {
    if (commandName !== "search_board_ports") {
      return Promise.reject(`unexpected command ${commandName}`);
    }
    return answerSearch();
  });
}

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

test("shows that it is searching until the search answers", () => {
  mockBoardPortSearch(() => new Promise<BoardPort[]>(() => undefined));
  render(<BoardPortList />);
  expect(screen.getByText("Searching for boards…")).toBeTruthy();
  expect(screen.getByRole("button", { name: "Search again" })).toHaveProperty("disabled", true);
});

test("shows each board port that the search finds", async () => {
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  render(<BoardPortList />);
  expect(await screen.findByText("/dev/cu.usbserial-0001")).toBeTruthy();
  expect(screen.getByText("CP210x")).toBeTruthy();
  expect(screen.getByText("CP2102 USB to UART Bridge Controller")).toBeTruthy();
  expect(screen.getByText("Serial number 0001")).toBeTruthy();
});

test("leaves out the serial number when the board does not report one", async () => {
  mockBoardPortSearch(() => Promise.resolve([{ ...cp210xBoardPort, serialNumber: null }]));
  render(<BoardPortList />);
  await screen.findByText("/dev/cu.usbserial-0001");
  expect(screen.queryByText(/^Serial number/)).toBeNull();
});

test("says how to connect a board when the search finds none", async () => {
  mockBoardPortSearch(() => Promise.resolve([]));
  render(<BoardPortList />);
  expect(await screen.findByText(/^No board found\./)).toBeTruthy();
});

test("explains when the system could not list its serial ports", async () => {
  mockBoardPortSearch(() => Promise.reject("port-enumeration-failed"));
  render(<BoardPortList />);
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The system could not list its serial ports.");
});

test("explains when the search stopped before it finished", async () => {
  mockBoardPortSearch(() => Promise.reject("search-interrupted"));
  render(<BoardPortList />);
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board search stopped before it finished.");
});

test("shows a general message and logs the cause for an unknown failure", async () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  mockBoardPortSearch(() => Promise.reject("Command search_board_ports not allowed by ACL"));
  render(<BoardPortList />);
  const failureParagraph = await screen.findByRole("alert");
  expect(failureParagraph.textContent).toBe("The board search failed.");
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    "The board search failed without a known code:",
    "Command search_board_ports not allowed by ACL",
  );
});

test("searches again when the button is pressed", async () => {
  mockBoardPortSearch(() => Promise.resolve([]));
  render(<BoardPortList />);
  await screen.findByText(/^No board found\./);
  mockBoardPortSearch(() => Promise.resolve([cp210xBoardPort]));
  fireEvent.click(screen.getByRole("button", { name: "Search again" }));
  expect(await screen.findByText("/dev/cu.usbserial-0001")).toBeTruthy();
});
