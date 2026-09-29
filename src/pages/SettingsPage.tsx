import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { Panel } from "../components/Panel";
import { useSettings } from "../hooks/useSettings";
import { settingsService } from "../services/settingsService";
import type { AppError } from "../types/errors";
import type { Settings } from "../types/settings";
import { toAppError } from "../utils/errors";

export function SettingsPage() {
  const { view, error: loadError, save } = useSettings();
  const [draft, setDraft] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  useEffect(() => {
    if (view) setDraft(view.settings);
  }, [view]);

  if (loadError) return <div className="page"><ErrorBanner error={loadError} title="Could not load settings" /></div>;
  if (!view || !draft) return <div className="page"><p className="muted">Loading settings…</p></div>;

  const update = (patch: Partial<Settings>) => {
    setDraft({ ...draft, ...patch });
    setSaved(false);
  };

  const browse = async () => {
    try {
      const dir = await settingsService.pickDirectory(draft.downloadDirectory ?? view.effectiveDownloadDirectory);
      if (dir) update({ downloadDirectory: dir });
    } catch (e) {
      setError(toAppError(e));
    }
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await save({
        ...draft,
        lastCameraAddress: draft.lastCameraAddress?.trim() || null,
        downloadDirectory: draft.downloadDirectory?.trim() || null,
      });
      setSaved(true);
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="page page-narrow">
      <h1>Settings</h1>
      <form className="form" onSubmit={submit}>
        <Panel title="Camera">
          <label className="field">
            <span>Camera address (used to prefill the connection screen)</span>
            <input
              className="mono"
              value={draft.lastCameraAddress ?? ""}
              placeholder={view.defaultCameraAddress}
              onChange={(e) => update({ lastCameraAddress: e.target.value })}
              spellCheck={false}
            />
          </label>
          <label className="checkbox">
            <input type="checkbox" checked={draft.mockMode} onChange={(e) => update({ mockMode: e.target.checked })} />
            Mock mode by default (simulated camera for development)
          </label>
          <label className="field field-inline">
            <span>Status refresh interval (seconds)</span>
            <input
              type="number"
              min={1}
              max={300}
              value={draft.statusPollIntervalSecs}
              onChange={(e) => update({ statusPollIntervalSecs: Number(e.target.value) })}
            />
          </label>
        </Panel>

        <Panel title="Downloads">
          <label className="field">
            <span>Download directory</span>
            <div className="input-row">
              <input
                className="mono"
                value={draft.downloadDirectory ?? ""}
                placeholder={view.effectiveDownloadDirectory}
                onChange={(e) => update({ downloadDirectory: e.target.value })}
                spellCheck={false}
              />
              <button type="button" className="btn" onClick={() => void browse()}>
                Browse…
              </button>
              <button type="button" className="btn btn-ghost" onClick={() => update({ downloadDirectory: null })}>
                Default
              </button>
            </div>
          </label>
          <p className="muted small">
            Files are organized as <code>YYYY-MM-DD/recording-name/</code> with the media file, FIT
            telemetry, thumbnail and <code>metadata.json</code>.
          </p>
        </Panel>

        <Panel title="Developer">
          <label className="checkbox">
            <input type="checkbox" checked={draft.debugMode} onChange={(e) => update({ debugMode: e.target.checked })} />
            Debug mode: show technical error details and raw camera JSON; verbose backend logging
          </label>
        </Panel>

        <div className="form-actions">
          <button type="submit" className="btn btn-primary" disabled={saving}>
            {saving ? "Saving…" : "Save settings"}
          </button>
          {saved && <span className="ok small">Saved</span>}
        </div>
        {error && <ErrorBanner error={error} title="Could not save settings" />}
      </form>
    </div>
  );
}
