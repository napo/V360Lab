import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { LanguageSelect } from "../components/LanguageSelect";
import { Panel } from "../components/Panel";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { settingsService } from "../services/settingsService";
import type { AppError } from "../types/errors";
import type { Settings } from "../types/settings";
import { toAppError } from "../utils/errors";

export function SettingsPage() {
  const { view, error: loadError, save } = useSettings();
  const { t } = useI18n();
  const [draft, setDraft] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  // Initialize the form once; later settings changes (e.g. the language,
  // which is saved immediately) must not discard unsaved edits.
  useEffect(() => {
    if (view && !draft) setDraft(view.settings);
  }, [view, draft]);

  if (loadError) {
    return (
      <div className="page">
        <ErrorBanner error={loadError} title={t("settings.loadFailed")} />
      </div>
    );
  }
  if (!view || !draft) {
    return (
      <div className="page">
        <p className="muted">{t("settings.loading")}</p>
      </div>
    );
  }

  const update = (patch: Partial<Settings>) => {
    setDraft({ ...draft, ...patch });
    setSaved(false);
  };

  const browse = async () => {
    try {
      const dir = await settingsService.pickDirectory(
        draft.downloadDirectory ?? view.effectiveDownloadDirectory,
        t("settings.chooseDirectory"),
      );
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
        // The language is managed by the selector above and saved on change.
        language: view.settings.language,
        // Chosen from the media viewer, saved there.
        detectionModel: view.settings.detectionModel,
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
      <h1>{t("settings.title")}</h1>
      <Panel title={t("settings.languagePanel")}>
        <label className="field field-inline">
          <span>{t("settings.language")}</span>
          <LanguageSelect includeSystem />
        </label>
      </Panel>
      <form className="form" onSubmit={submit}>
        <Panel title={t("settings.cameraPanel")}>
          <label className="field">
            <span>{t("settings.cameraAddress")}</span>
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
            {t("settings.mockDefault")}
          </label>
          <label className="field field-inline">
            <span>{t("settings.pollInterval")}</span>
            <input
              type="number"
              min={1}
              max={300}
              value={draft.statusPollIntervalSecs}
              onChange={(e) => update({ statusPollIntervalSecs: Number(e.target.value) })}
            />
          </label>
        </Panel>

        <Panel title={t("settings.downloadsPanel")}>
          <label className="field">
            <span>{t("settings.downloadDirectory")}</span>
            <div className="input-row">
              <input
                className="mono"
                value={draft.downloadDirectory ?? ""}
                placeholder={view.effectiveDownloadDirectory}
                onChange={(e) => update({ downloadDirectory: e.target.value })}
                spellCheck={false}
              />
              <button type="button" className="btn" onClick={() => void browse()}>
                {t("settings.browse")}
              </button>
              <button type="button" className="btn btn-ghost" onClick={() => update({ downloadDirectory: null })}>
                {t("settings.default")}
              </button>
            </div>
          </label>
          <p className="muted small">
            <LayoutNote />
          </p>
        </Panel>

        <Panel title={t("settings.developerPanel")}>
          <label className="checkbox">
            <input type="checkbox" checked={draft.debugMode} onChange={(e) => update({ debugMode: e.target.checked })} />
            {t("settings.debugMode")}
          </label>
        </Panel>

        <div className="form-actions">
          <button type="submit" className="btn btn-primary" disabled={saving}>
            {saving ? t("settings.saving") : t("settings.save")}
          </button>
          {saved && <span className="ok small">{t("settings.saved")}</span>}
        </div>
        {error && <ErrorBanner error={error} title={t("settings.saveFailed")} />}
      </form>
    </div>
  );
}

function LayoutNote() {
  const { tx } = useI18n();
  return tx("settings.layout", {
    pattern: <code>YYYY-MM-DD/recording-name/</code>,
    metadata: <code>metadata.json</code>,
  });
}
