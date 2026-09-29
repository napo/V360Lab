import { useSettings } from "../hooks/useSettings";
import type { AppError } from "../types/errors";

interface ErrorBannerProps {
  error: AppError;
  title?: string;
  onRetry?: () => void;
}

/** Readable error message; technical details only in debug mode. */
export function ErrorBanner({ error, title, onRetry }: ErrorBannerProps) {
  const { debugMode } = useSettings();
  return (
    <div className="banner banner-error" role="alert">
      <div className="banner-main">
        <strong>{title ?? "Error"}</strong>
        <span>{error.message}</span>
        {onRetry && (
          <button type="button" className="btn btn-small" onClick={onRetry}>
            Retry
          </button>
        )}
      </div>
      {debugMode && (
        <details className="banner-detail">
          <summary>Technical details</summary>
          <pre>{`kind: ${error.kind}${error.detail ? `\n${error.detail}` : ""}`}</pre>
        </details>
      )}
    </div>
  );
}
