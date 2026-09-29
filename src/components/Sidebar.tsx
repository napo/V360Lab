import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { ConnectionBadge } from "./ConnectionBadge";
import { LanguageSelect } from "./LanguageSelect";
import { PAGES, type PageId } from "./navigation";

interface SidebarProps {
  current: PageId;
  onNavigate: (page: PageId) => void;
}

export function Sidebar({ current, onNavigate }: SidebarProps) {
  const { connection, disconnect } = useCamera();
  const { t } = useI18n();

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/v360lab.svg" alt="" width={28} height={28} />
        <span>{t("app.name")}</span>
      </div>
      <nav>
        {PAGES.map((page) => (
          <button
            key={page.id}
            type="button"
            className={`nav-item ${page.id === current ? "active" : ""}`}
            onClick={() => onNavigate(page.id)}
          >
            {t(page.labelKey)}
          </button>
        ))}
      </nav>
      <div className="sidebar-footer">
        <ConnectionBadge connection={connection} />
        {connection.status === "connected" && (
          <>
            <div className="sidebar-address mono" title={connection.address}>
              {connection.address}
            </div>
            <button type="button" className="btn btn-small btn-ghost" onClick={() => void disconnect()}>
              {t("sidebar.disconnect")}
            </button>
          </>
        )}
        <LanguageSelect className="sidebar-language" />
      </div>
    </aside>
  );
}
