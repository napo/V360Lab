import { useState, type ComponentType } from "react";
import { PAGES, type PageId } from "./components/navigation";
import { Sidebar } from "./components/Sidebar";
import { CameraProvider } from "./context/CameraContext";
import { DownloadsProvider } from "./context/DownloadsContext";
import { SettingsProvider } from "./context/SettingsContext";
import { useCamera } from "./hooks/useCamera";
import { I18nProvider } from "./i18n/I18nContext";
import { AboutPage } from "./pages/AboutPage";
import { ConnectionPage } from "./pages/ConnectionPage";
import { DashboardPage } from "./pages/DashboardPage";
import { FeaturesPage } from "./pages/FeaturesPage";
import { MediaPage } from "./pages/MediaPage";
import { SettingsPage } from "./pages/SettingsPage";

const PAGE_COMPONENTS: Record<PageId, ComponentType> = {
  dashboard: DashboardPage,
  media: MediaPage,
  features: FeaturesPage,
  settings: SettingsPage,
  about: AboutPage,
};

function Shell() {
  const [page, setPage] = useState<PageId>("dashboard");
  const { connection } = useCamera();
  const definition = PAGES.find((p) => p.id === page)!;
  const Page =
    definition.requiresCamera && connection.status !== "connected"
      ? ConnectionPage
      : PAGE_COMPONENTS[page];

  return (
    <div className="app">
      <Sidebar current={page} onNavigate={setPage} />
      <main className="content">
        <Page />
      </main>
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
