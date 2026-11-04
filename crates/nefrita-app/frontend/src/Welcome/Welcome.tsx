export function Welcome({ onConnectDevice }: { onConnectDevice: () => void }) {
  return (
    <section>
      <p>Nefrita keeps your XBT keys on an ESP32 board, away from this computer.</p>
      <button type="button" onClick={onConnectDevice}>
        Connect a board
      </button>
    </section>
  );
}
