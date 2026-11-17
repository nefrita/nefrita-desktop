export const noConnectedDeviceMessage =
  "The app is no longer connected to the board. Connect it again.";

export const connectionLostMessage =
  "The connection to the board was lost. Check the cable and try again.";

export const deviceNotRecognizedMessage =
  "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.";

export const commandInterruptedMessage = "The app stopped before it finished talking to the board.";

export function isCommandError<CommandError extends string>(
  commandFailure: unknown,
  errorMessages: Record<CommandError, string>,
): commandFailure is CommandError {
  return typeof commandFailure === "string" && Object.hasOwn(errorMessages, commandFailure);
}

export function commandFailureMessage<CommandError extends string>(
  commandFailure: unknown,
  errorMessages: Record<CommandError, string>,
  unknownFailureMessage: string,
): string {
  if (isCommandError(commandFailure, errorMessages)) {
    return errorMessages[commandFailure];
  }
  console.error(unknownFailureMessage, commandFailure);
  return unknownFailureMessage;
}
