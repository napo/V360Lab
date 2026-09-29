import { useEffect, useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { FeatureTable } from "../components/features/FeatureTable";
import { JsonViewer } from "../components/JsonViewer";
import { useCamera } from "../hooks/useCamera";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { featureLabel } from "../i18n/featureLabels";

/** Camera features (settings) with inline editing. */
export function FeaturesPage() {
  const { features, featuresError, featuresLoading, featureError, loadFeatures, status } = useCamera();
  const { debugMode } = useSettings();
  const { t, lookup } = useI18n();
  const [query, setQuery] = useState("");
  const [showRaw, setShowRaw] = useState(false);

  useEffect(() => {
    if (!features && !featuresLoading) void loadFeatures();
    // Load once when the page opens without data.
  }, []);

  const filtered = useMemo(() => {
    const list = features?.features ?? [];
    const q = query.trim().toLowerCase();
    if (!q) return list;
    return list.filter((f) =>
      [f.key, featureLabel(lookup, f), JSON.stringify(f.value)].some((text) =>
        text.toLowerCase().includes(q),
      ),
    );
  }, [features, query, lookup]);

  return (
    <div className="page">
      <h1>{t("features.title")}</h1>
      <div className="toolbar">
        <button type="button" className="btn" disabled={featuresLoading} onClick={() => void loadFeatures()}>
          {featuresLoading ? t("common.loading") : t("common.refresh")}
        </button>
        <input
          type="search"
          placeholder={t("features.filter")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <label className="checkbox">
          <input
            type="checkbox"
            checked={showRaw || debugMode}
            disabled={debugMode}
            onChange={(e) => setShowRaw(e.target.checked)}
          />
          {t("features.showRaw")}
        </label>
        <span className="toolbar-spacer" />
        {features && (
          <span className="muted small">{t("features.count", { count: features.features.length })}</span>
        )}
      </div>
      <p className="muted small">
        {t("features.editHint")} {t("features.readOnly")}
      </p>
      {status?.recordingState === "recording" && (
        <p className="muted small">{t("capture.lockedWhileRecording")}</p>
      )}
      {featuresError && (
        <ErrorBanner error={featuresError} title={t("features.loadFailed")} onRetry={() => void loadFeatures()} />
      )}
      {featureError && <ErrorBanner error={featureError} title={t("capture.settingFailed")} />}
      {features && <FeatureTable features={filtered} />}
      {features && (showRaw || debugMode) && (
        <JsonViewer value={features.raw} title={t("features.raw")} defaultOpen />
      )}
    </div>
  );
}
