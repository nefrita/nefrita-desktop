import { useCallback, useState } from "react";
import type { ConnectedDevice } from "../bindings/ConnectedDevice";
import type { DeviceConnectionError } from "../bindings/DeviceConnectionError";
import { connectDevice, disconnectDevice } from "../commands/deviceConnection";
import { BoardPortList } from "./BoardPortList";
import { ConnectedDeviceDetails } from "./ConnectedDeviceDetails";

type ConnectionProgress =
  | { stage: "choosing"; failureMessage: string | null }
  | { stage: "connecting"; portName: string }
  | { stage: "connected"; connectedDevice: ConnectedDevice };

const choosingABoard: ConnectionProgress = { stage: "choosing", failureMessage: null };

const connectionErrorMessages: Record<DeviceConnectionError, string> = {
  "port-enumeration-failed": "The system could not list its serial ports.",
  "port-not-listed": "The board is no longer in the list. Search again and choose it.",
  "port-unavailable":
    "The port is busy or the board was unplugged. Close other programs that use it and try again.",
  "port-access-denied":
    "The system did not let the app open the port. On Linux, your user needs the group of the port, such as dialout.",
  "port-open-failed": "The port could not be opened.",
  "device-not-answering": "The board did not answer. Check that it runs the Nefrita firmware.",
  "device-not-recognized":
    "The board did not answer like a Nefrita board. Check that it runs the Nefrita firmware.",
  "not-xbt-firmware": "The board does not run the XBT firmware of Nefrita.",
  "connection-lost": "The connection to the board was lost. Check the cable and try again.",
  "command-interrupted": "The app stopped before it finished talking to the board.",
};

const unknownConnectionFailureMessage = "The connection to the board failed.";

const unknownDisconnectionFailureMessage = "The board could not be disconnected.";

const unpluggedMessage = "The board was unplugged.";

function isDeviceConnectionError(commandFailure: unknown): commandFailure is DeviceConnectionError {
  return (
    typeof commandFailure === "string" && Object.hasOwn(connectionErrorMessages, commandFailure)
  );
}

function commandFailureMessage(commandFailure: unknown, unknownFailureMessage: string): string {
  if (isDeviceConnectionError(commandFailure)) {
    return connectionErrorMessages[commandFailure];
  }
  console.error(unknownFailureMessage, commandFailure);
  return unknownFailureMessage;
}

export function DeviceConnection() {
  const [connectionProgress, setConnectionProgress] = useState<ConnectionProgress>(choosingABoard);

  async function connectToPort(portName: string) {
    setConnectionProgress({ stage: "connecting", portName });
    try {
      const connectedDevice = await connectDevice(portName);
      setConnectionProgress({ stage: "connected", connectedDevice });
    } catch (connectionFailure) {
      setConnectionProgress({
        stage: "choosing",
        failureMessage: commandFailureMessage(connectionFailure, unknownConnectionFailureMessage),
      });
    }
  }

  const disconnectAndChooseAgain = useCallback(async (failureMessage: string | null) => {
    try {
      await disconnectDevice();
      setConnectionProgress({ stage: "choosing", failureMessage });
    } catch (disconnectionFailure) {
      setConnectionProgress({
        stage: "choosing",
        failureMessage: commandFailureMessage(
          disconnectionFailure,
          unknownDisconnectionFailureMessage,
        ),
      });
    }
  }, []);

  const disconnectAfterUnplugging = useCallback(
    () => disconnectAndChooseAgain(unpluggedMessage),
    [disconnectAndChooseAgain],
  );

  switch (connectionProgress.stage) {
    case "choosing":
      return (
        <>
          {connectionProgress.failureMessage !== null && (
            <p role="alert">{connectionProgress.failureMessage}</p>
          )}
          <BoardPortList onConnect={connectToPort} />
        </>
      );
    case "connecting":
      return <p>Connecting to {connectionProgress.portName}…</p>;
    case "connected":
      return (
        <ConnectedDeviceDetails
          connectedDevice={connectionProgress.connectedDevice}
          onDisconnect={() => disconnectAndChooseAgain(null)}
          onUnplugged={disconnectAfterUnplugging}
        />
      );
  }
}
