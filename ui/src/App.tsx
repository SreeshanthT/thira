import { useEffect, useState } from "react";
import { appVersion } from "./ipc";

// Placeholder shell; the panel layout arrives in #15.
export function App() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    appVersion().then(setVersion, () => setVersion(null));
  }, []);

  return (
    <main className="app">
      <h1>Thira</h1>
      {version && <p className="version">v{version}</p>}
    </main>
  );
}
