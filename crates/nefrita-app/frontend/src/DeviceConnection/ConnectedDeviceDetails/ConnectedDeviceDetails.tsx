import { useEffect } from "react";
import type { ConnectedDevice } from "../../bindings/ConnectedDevice";
import type { ConnectedDeviceState } from "../../bindings/ConnectedDeviceState";
import { searchBoardPorts } from "../../commands/boardPorts";

const portCheckIntervalMilliseconds = 2000;

const deviceStateLabels: Record<ConnectedDeviceState, string> = {
  uninitialized: "No wallet yet",
  unsaved: "Wallet not saved",
  locked: "Locked",
  ready: "Unlocked",
  temporary: "Temporary wallet",
};

export function ConnectedDeviceDetails({
  connectedDevice,
  onDisconnect,
  onUnplugged,
}: {
  connectedDevice: ConnectedDevice;
  onDisconnect: () => void;
  onUnplugged: () => void;
}) {
  useEffect(() => {
    let portChecksStopped = false;
    const portCheckTimer = setInterval(async () => {
      try {
        const boardPorts = await searchBoardPorts();
        const portStillListed = boardPorts.some(
          (boardPort) => boardPort.portName === connectedDevice.portName,
        );
        if (!portChecksStopped && !portStillListed) {
          stopPortChecks();
          onUnplugged();
        }
      } catch (searchFailure) {
        console.error("The check of the connected board failed:", searchFailure);
      }
    }, portCheckIntervalMilliseconds);

    function stopPortChecks() {
      portChecksStopped = true;
      clearInterval(portCheckTimer);
    }

    return stopPortChecks;
  }, [connectedDevice.portName, onUnplugged]);

  return (
    <section>
      <h2>Connected board</h2>
      <dl>
        <dt>Port</dt>
        <dd>{connectedDevice.portName}</dd>
        <dt>Firmware</dt>
        <dd>{connectedDevice.firmwareVersion}</dd>
        <dt>Board</dt>
        <dd>{connectedDevice.boardType}</dd>
        <dt>State</dt>
        <dd>{deviceStateLabels[connectedDevice.deviceState]}</dd>
      </dl>
      <button type="button" onClick={onDisconnect}>
        Disconnect
      </button>
    </section>
  );
}
