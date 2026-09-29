import { useI18n } from "../../hooks/useI18n";
import type { DeleteReport } from "../../types/downloads";
import { ErrorBanner } from "../ErrorBanner";

export function DeleteResult({ report }: { report: DeleteReport }) {
  const { t } = useI18n();
  return (
    <div className="delete-result">
      {report.deleted.length > 0 && (
        <p className="ok small">{t("media.deleted", { count: report.deleted.length })}</p>
      )}
      {!report.verified && <p className="warning small">{t("media.deleteUnverified")}</p>}
      {report.failed.map((failure) => (
        <ErrorBanner
          key={failure.itemId}
          error={failure.error}
          title={t("media.deleteFailed", { name: failure.name })}
        />
      ))}
    </div>
  );
}
