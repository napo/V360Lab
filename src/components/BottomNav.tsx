import { useI18n } from "../hooks/useI18n";
import { PageIcon } from "./icons";
import { PAGES, type PageId } from "./navigation";

interface BottomNavProps {
  current: PageId;
  onNavigate: (page: PageId) => void;
}

/** Phone navigation: four large targets at the bottom of the screen. */
export function BottomNav({ current, onNavigate }: BottomNavProps) {
  const { t } = useI18n();
  return (
    <nav className="bottom-nav">
      {PAGES.map((page) => (
        <button
          key={page.id}
          type="button"
          className={page.id === current ? "active" : undefined}
          aria-current={page.id === current ? "page" : undefined}
          onClick={() => onNavigate(page.id)}
        >
          <PageIcon page={page.id} />
          <span>{t(page.labelKey)}</span>
        </button>
      ))}
    </nav>
  );
}
