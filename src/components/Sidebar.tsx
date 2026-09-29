import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { ConnectionBadge } from "./ConnectionBadge";
import { PageIcon } from "./icons";
import { LanguageSelect } from "./LanguageSelect";
import { PAGES, type PageId } from "./navigation";

interface SidebarProps {
  current: PageId;
  onNavigate: (page: PageId) => void;
}

/** Desktop navigation. */
export function Sidebar({ current, onNavigate }: SidebarProps) {
  const { connection, disconnect } = useCamera();
  const { t } = useI18n();

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/brand/logo-dark-bg.png" alt={t("app.name")} />
      </div>
      <nav>
        {PAGES.map((page) => (
          <button
            key={page.id}
            type="button"
            className={`nav-item ${page.id === current ? "active" : ""}`}
            onClick={() => onNavigate(page.id)}
          >
            <PageIcon page={page.id} size={20} />
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
        <label className="sidebar-language-field">
          {t("sidebar.language")}
          <LanguageSelect className="sidebar-language" />
        </label>
      </div>
    </aside>
  );
}
