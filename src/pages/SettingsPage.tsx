import { useEffect, useState, type FormEvent } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { LanguageSelect } from "../components/LanguageSelect";
import { ModelManager } from "../components/ModelManager";
import { Panel } from "../components/Panel";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { settingsService } from "../services/settingsService";
import { updateService } from "../services/updateService";
import type { AppError } from "../types/errors";
import type { Settings, UpdateInfo } from "../types/settings";
import { toAppError } from "../utils/errors";

export function SettingsPage() {
  const { view, error: loadError, save } = useSettings();
  const { t } = useI18n();
  const [draft, setDraft] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateInfo | null>(null);
  const [updateError, setUpdateError] = useState<AppError | null>(null);

  const checkNow = async () => {
    setCheckingUpdate(true);
    setUpdateError(null);
    try {
      setUpdateResult(await updateService.check());
      updateService.markChecked();
    } catch (e) {
      setUpdateError(toAppError(e));
    } finally {
      setCheckingUpdate(false);
    }
  };

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
      <Panel title={t("models.panel")}>
        <p className="muted small">{t("models.intro")}</p>
        <ModelManager />
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

        <Panel title={t("settings.updatesPanel")}>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={draft.checkUpdates}
              onChange={(e) => update({ checkUpdates: e.target.checked })}
            />
            {t("settings.checkUpdates")}
          </label>
          <p className="muted small">{t("settings.checkUpdatesHint")}</p>
          <div className="input-row">
            <button type="button" className="btn btn-small" disabled={checkingUpdate} onClick={() => void checkNow()}>
              {checkingUpdate ? t("settings.checkingUpdate") : t("settings.checkNow")}
            </button>
            {updateResult && (
              <span className="small">
                {updateResult.available
                  ? t("settings.updateAvailable", { version: updateResult.latestVersion })
                  : t("settings.upToDate", { version: updateResult.currentVersion })}
              </span>
            )}
            {updateResult?.available && (
              <button type="button" className="btn btn-small btn-ghost" onClick={() => void updateService.open(updateResult.pageUrl)}>
                {t("update.details")}
              </button>
            )}
          </div>
          {updateError && <ErrorBanner error={updateError} title={t("update.failed")} />}
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
