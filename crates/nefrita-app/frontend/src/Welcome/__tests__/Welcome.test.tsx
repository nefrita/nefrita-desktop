import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { Welcome } from "../Welcome";

afterEach(() => {
  cleanup();
});

test("says what Nefrita does", () => {
  render(<Welcome onConnectDevice={() => undefined} />);
  expect(screen.getByText(/^Nefrita keeps your XBT keys/)).toBeTruthy();
});

test("asks to connect a board when the button is pressed", () => {
  const onConnectDevice = vi.fn();
  render(<Welcome onConnectDevice={onConnectDevice} />);
  fireEvent.click(screen.getByRole("button", { name: "Connect a board" }));
  expect(onConnectDevice).toHaveBeenCalledOnce();
});
