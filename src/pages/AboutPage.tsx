import { Panel } from "../components/Panel";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { useI18n } from "../hooks/useI18n";
import type { TranslationKey } from "../i18n/types";
import { settingsService } from "../services/settingsService";

const ROADMAP: TranslationKey[] = [
  "about.roadmap1",
  "about.roadmap2",
  "about.roadmap3",
  "about.roadmap4",
  "about.roadmap5",
];

export function AboutPage() {
  const info = useAsyncResource(settingsService.appInfo);
  const { t } = useI18n();

  const version = info.data
    ? `${t("about.version", { version: info.data.version })}${info.data.debugBuild ? ` ${t("about.debugBuild")}` : ""}`
    : info.error
      ? t("about.versionUnavailable")
      : "…";

  return (
    <div className="page page-narrow">
      <h1>{t("about.title")}</h1>
      <Panel title={t("app.name")}>
        <p>{t("about.intro")}</p>
        <p className="mono small">{version}</p>
        <p>{t("about.license")}</p>
      </Panel>
      <Panel title={t("about.roadmap")}>
        <ol className="hint-list">
          {ROADMAP.map((key) => (
            <li key={key}>{t(key)}</li>
          ))}
        </ol>
      </Panel>
      <Panel title={t("about.disclaimerTitle")}>
        <p className="small">{t("about.disclaimer")}</p>
      </Panel>
    </div>
  );
}
