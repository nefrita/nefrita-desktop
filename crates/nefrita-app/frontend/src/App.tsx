import { useState } from "react";
import { DeviceConnection } from "./DeviceConnection";
import { Welcome } from "./Welcome";

type AppView = "welcome" | "device-connection";

export function App() {
  const [currentView, setCurrentView] = useState<AppView>("welcome");

  return (
    <main>
      <h1>Nefrita</h1>
      {currentView === "welcome" ? (
        <Welcome onConnectDevice={() => setCurrentView("device-connection")} />
      ) : (
        <DeviceConnection />
      )}
    </main>
  );
}
