import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import type { AppError } from "../types/errors";

interface ErrorBannerProps {
  error: AppError;
  title?: string;
  onRetry?: () => void;
}

/** Translated error message; technical details only in debug mode. */
export function ErrorBanner({ error, title, onRetry }: ErrorBannerProps) {
  const { debugMode } = useSettings();
  const { t, errorMessage } = useI18n();
  const technical = [`kind: ${error.kind}`, error.message, error.detail].filter(Boolean).join("\n");

  return (
    <div className="banner banner-error" role="alert">
      <div className="banner-main">
        <strong>{title ?? t("common.error")}</strong>
        <span>{errorMessage(error)}</span>
        {onRetry && (
          <button type="button" className="btn btn-small" onClick={onRetry}>
            {t("common.retry")}
          </button>
        )}
      </div>
      {debugMode && (
        <details className="banner-detail">
          <summary>{t("common.technicalDetails")}</summary>
          <pre>{technical}</pre>
        </details>
      )}
    </div>
  );
}
