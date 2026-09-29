import { useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { FeatureTable } from "../components/features/FeatureTable";
import { JsonViewer } from "../components/JsonViewer";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";

/** Read-only view of the features/settings reported by the camera. */
export function FeaturesPage() {
  const features = useAsyncResource(cameraService.features);
  const { debugMode } = useSettings();
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
      <h1>Camera Features</h1>
      <div className="toolbar">
        <button type="button" className="btn" disabled={features.loading} onClick={() => void features.reload()}>
          {features.loading ? "Loading…" : "Refresh"}
        </button>
        <input
          type="search"
          placeholder="Filter features…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <label className="checkbox">
          <input type="checkbox" checked={showRaw || debugMode} disabled={debugMode} onChange={(e) => setShowRaw(e.target.checked)} />
          Show raw JSON
        </label>
        <span className="toolbar-spacer" />
        {features.data && (
          <span className="muted small">{features.data.features.length} features reported</span>
        )}
      </div>
      <p className="muted small">
        Values are shown as reported by the camera. Editing settings is not supported yet.
      </p>
      {features.error && (
        <ErrorBanner error={features.error} title="Could not load features" onRetry={() => void features.reload()} />
      )}
      {features.data && <FeatureTable features={filtered} />}
      {features.data && (showRaw || debugMode) && (
        <JsonViewer value={features.data.raw} title="Raw features response" defaultOpen />
      )}
    </div>
  );
}
