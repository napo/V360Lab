import { useEffect, useRef, useState } from "react";
import { BottomNav } from "./components/BottomNav";
import { MobileHeader } from "./components/MobileHeader";
import { PAGES, type PageId } from "./components/navigation";
import { NotConnected } from "./components/NotConnected";
import { Sidebar } from "./components/Sidebar";
import { CameraProvider } from "./context/CameraContext";
import { DownloadsProvider } from "./context/DownloadsContext";
import { SettingsProvider } from "./context/SettingsContext";
import { useCamera } from "./hooks/useCamera";
import { I18nProvider } from "./i18n/I18nContext";
import { UpdateNotice } from "./components/UpdateNotice";
import { AdvancedPage } from "./pages/AdvancedPage";
import { CapturePage } from "./pages/CapturePage";
import { ConnectPage } from "./pages/ConnectPage";
import { MediaPage } from "./pages/MediaPage";

function Shell() {
  const [page, setPage] = useState<PageId>("connect");
  const { connection } = useCamera();
  const connected = connection.status === "connected";

  // Once connected from the Connect section, go straight to capturing.
  const wasConnected = useRef(connected);
  useEffect(() => {
    if (connected && !wasConnected.current && page === "connect") setPage("capture");
    wasConnected.current = connected;
  }, [connected, page]);

  const definition = PAGES.find((p) => p.id === page)!;
  let content;
  if (page === "connect") content = <ConnectPage navigate={setPage} />;
  else if (page === "capture") content = <CapturePage navigate={setPage} />;
  else if (page === "advanced") content = <AdvancedPage navigate={setPage} />;
  else if (definition.requiresCamera && !connected) content = <NotConnected navigate={setPage} />;
  else content = <MediaPage />;

  return (
    <div className="app">
      <Sidebar current={page} onNavigate={setPage} />
      <div className="main">
        <MobileHeader />
        <main className="content">
          <UpdateNotice />
          {content}
        </main>
        <BottomNav current={page} onNavigate={setPage} />
      </div>
    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <I18nProvider>
        <CameraProvider>
          <DownloadsProvider>
            <Shell />
          </DownloadsProvider>
        </CameraProvider>
      </I18nProvider>
    </SettingsProvider>
  );
}
