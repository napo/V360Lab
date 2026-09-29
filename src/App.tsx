import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  name: string;
  version: string;
  debug: boolean;
}

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppInfo>("app_info").then(setInfo).catch((e) => setError(String(e)));
  }, []);

  return (
    <main>
      <h1>V360Lab</h1>
      {info && (
        <p>
          Backend: {info.name} {info.version}
        </p>
      )}
      {error && <p>Backend unavailable: {error}</p>}
    </main>
  );
}
