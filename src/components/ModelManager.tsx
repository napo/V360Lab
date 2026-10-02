import { useEffect, useState } from "react";
import { useI18n } from "../hooks/useI18n";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";
import { settingsService } from "../services/settingsService";
import type { ModelDownloadProgress, ModelStatus } from "../types/camera";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";
import { formatBytes } from "../utils/format";
import { ErrorBanner } from "./ErrorBanner";

type NavigatorWithConnection = Navigator & { connection?: { type?: string } };

/** True when the device says it is on mobile data (Android only). */
function onMobileData(): boolean {
  return (navigator as NavigatorWithConnection).connection?.type === "cellular";
}

/**
 * The object detection model: offers to download the recommended one
 * (checking its size and the free space first, and reminding to use a
 * Wi-Fi with internet), or to choose a custom ONNX file.
 */
export function ModelManager() {
  const { t } = useI18n();
  const { view, save, reload } = useSettings();
  const [models, setModels] = useState<ModelStatus[] | null>(null);
  const [progress, setProgress] = useState<ModelDownloadProgress | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const active = view?.settings.detectionModel ?? null;

  const load = async () => {
    try {
      setModels(await cameraService.listModels());
    } catch (e) {
      setError(toAppError(e));
    }
  };

  useEffect(() => {
    void load();
  }, [active]);

  const download = async (model: ModelStatus) => {
    setError(null);
    setChecking(true);
    try {
      const check = await cameraService.checkModelDownload(model.id);
      if (!check.enoughSpace) {
        setError(
          toAppError({
            kind: "modelDownload",
            message: "not enough space",
            detail: null,
            params: { reason: "noSpace" },
          }),
        );
        return;
      }
      const lines = [
        t("models.confirmSize", { name: model.name, size: formatBytes(model.sizeBytes) }),
        check.freeBytes !== null ? t("models.confirmFree", { free: formatBytes(check.freeBytes) }) : null,
        t("models.confirmWifi"),
        onMobileData() ? t("models.confirmMobileData") : null,
        t("models.confirmLicense", { license: model.license }),
      ].filter(Boolean);
      const confirmed = await settingsService.confirm(lines.join("\n\n"), {
        title: t("models.confirmTitle"),
        okLabel: t("models.download"),
        cancelLabel: t("common.cancel"),
      });
      if (!confirmed) return;
      setChecking(false);
      setProgress({ received: 0, total: model.sizeBytes });
      await cameraService.downloadModel(model.id, setProgress);
      await reload();
      await load();
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setChecking(false);
      setProgress(null);
    }
  };

  const use = async (path: string) => {
    if (view) await save({ ...view.settings, detectionModel: path });
  };

  const chooseFile = async () => {
    try {
      const path = await settingsService.pickModel(t("detect.chooseModel"));
      if (path) await use(path);
    } catch (e) {
      setError(toAppError(e));
    }
  };

  const busy = checking || progress !== null;
  const custom = active && !models?.some((m) => m.installedPath === active);

  return (
    <div className="model-manager">
      {models?.map((model) => (
        <div key={model.id} className="model-row">
          <div>
            <strong>{model.name}</strong>
            <div className="muted small">
              {t("models.description", { classes: model.classes, size: formatBytes(model.sizeBytes) })} ·{" "}
              {model.license}
            </div>
          </div>
          <span className="toolbar-spacer" />
          {model.active ? (
            <span className="badge">{t("models.inUse")}</span>
          ) : model.installedPath ? (
            <button type="button" className="btn btn-small" onClick={() => void use(model.installedPath!)}>
              {t("models.use")}
            </button>
          ) : (
            <button type="button" className="btn btn-small btn-primary" disabled={busy} onClick={() => void download(model)}>
              {checking ? t("models.checking") : t("models.download")}
            </button>
          )}
        </div>
      ))}
      {progress && (
        <div className="frame-progress">
          <progress value={progress.received} max={progress.total || 1} />
          <span className="mono small">
            {formatBytes(progress.received)} / {formatBytes(progress.total)}
          </span>
        </div>
      )}
      {custom && (
        <p className="small">
          {t("detect.model")}: <span className="mono selectable">{active}</span>
        </p>
      )}
      <p className="muted small">
        {t("models.customHint")}{" "}
        <button type="button" className="btn btn-small btn-ghost" disabled={busy} onClick={() => void chooseFile()}>
          {t("models.chooseFile")}
        </button>
      </p>
      {error && <ErrorBanner error={error} title={t("models.failed")} />}
    </div>
  );
}
