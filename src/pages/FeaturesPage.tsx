import { useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { FeatureTable } from "../components/features/FeatureTable";
import { JsonViewer } from "../components/JsonViewer";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";

/** Read-only view of the features/settings reported by the camera. */
export function FeaturesPage() {
  const features = useAsyncResource(cameraService.features);
  const { debugMode } = useSettings();
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [showRaw, setShowRaw] = useState(false);

  const filtered = useMemo(() => {
    const list = features.data?.features ?? [];
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((f) =>
      [f.key, f.label ?? "", JSON.stringify(f.value)].some((text) => text.toLowerCase().includes(q)),
    );
  }, [features.data, query]);

  return (
    <div className="page">
      <h1>{t("features.title")}</h1>
      <div className="toolbar">
        <button type="button" className="btn" disabled={features.loading} onClick={() => void features.reload()}>
          {features.loading ? t("common.loading") : t("common.refresh")}
        </button>
        <input
          type="search"
          placeholder={t("features.filter")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <label className="checkbox">
          <input type="checkbox" checked={showRaw || debugMode} disabled={debugMode} onChange={(e) => setShowRaw(e.target.checked)} />
          {t("features.showRaw")}
        </label>
        <span className="toolbar-spacer" />
        {features.data && (
          <span className="muted small">
            {t("features.count", { count: features.data.features.length })}
          </span>
        )}
      </div>
      <p className="muted small">
        {t("features.readOnly")}
      </p>
      {features.error && (
        <ErrorBanner error={features.error} title={t("features.loadFailed")} onRetry={() => void features.reload()} />
      )}
      {features.data && <FeatureTable features={filtered} />}
      {features.data && (showRaw || debugMode) && (
        <JsonViewer value={features.data.raw} title={t("features.raw")} defaultOpen />
      )}
    </div>
  );
}
