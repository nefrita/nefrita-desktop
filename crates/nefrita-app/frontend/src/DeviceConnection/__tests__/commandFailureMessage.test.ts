import { afterEach, expect, test, vi } from "vitest";
import type { BoardPortSearchError } from "../../bindings/BoardPortSearchError";
import { commandFailureMessage, isCommandError } from "../commandFailureMessage";

const portEnumerationFailedMessage = "The system could not list its serial ports.";

const unknownSearchFailureMessage = "The board search failed.";

const searchErrorMessages: Record<BoardPortSearchError, string> = {
  "port-enumeration-failed": portEnumerationFailedMessage,
  "search-interrupted": "The board search stopped before it finished.",
};

const searchFailuresWithoutAMessage: unknown[] = ["port-not-listed", "toString", 404, null];

function searchFailureMessage(searchFailure: unknown): string {
  return commandFailureMessage(searchFailure, searchErrorMessages, unknownSearchFailureMessage);
}

afterEach(() => {
  vi.restoreAllMocks();
});

test("recognizes a code that has a message", () => {
  expect(isCommandError("search-interrupted", searchErrorMessages)).toBe(true);
});

test.each(searchFailuresWithoutAMessage)(
  "does not take %s for a code that has a message",
  (searchFailure) => {
    expect(isCommandError(searchFailure, searchErrorMessages)).toBe(false);
  },
);

test("gives the message of a known code without logging it", () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  expect(searchFailureMessage("port-enumeration-failed")).toBe(portEnumerationFailedMessage);
  expect(consoleErrorSpy).not.toHaveBeenCalled();
});

test("gives the general message and logs the cause of an unknown failure", () => {
  const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);
  expect(searchFailureMessage("Command search_board_ports not allowed by ACL")).toBe(
    unknownSearchFailureMessage,
  );
  expect(consoleErrorSpy).toHaveBeenCalledWith(
    unknownSearchFailureMessage,
    "Command search_board_ports not allowed by ACL",
  );
});
