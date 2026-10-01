import { useState } from "react";
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

/** Where the source code of this version can be obtained (AGPL-3.0). */
const SOURCE_URL = "https://github.com/napo/V360Lab";
const AUTHOR = "Maurizio Napolitano";
const FIRST_YEAR = 2026;

export function AboutPage() {
  const info = useAsyncResource(settingsService.appInfo);
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const year = new Date().getFullYear();
  const years = year > FIRST_YEAR ? `${FIRST_YEAR}–${year}` : `${FIRST_YEAR}`;

  const copySource = async () => {
    try {
      await navigator.clipboard.writeText(SOURCE_URL);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // The address stays selectable.
    }
  };

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
        <p>
          <strong>{t("about.authorLabel")}:</strong> {AUTHOR}
        </p>
        <p>{t("about.author", { author: AUTHOR, years })}</p>
        <p>{t("about.license")}</p>
      </Panel>
      <Panel title={t("about.sourceTitle")}>
        <p>{t("about.source")}</p>
        <div className="input-row">
          <span className="mono selectable">{SOURCE_URL}</span>
          <button type="button" className="btn btn-small" onClick={() => void copySource()}>
            {copied ? t("common.copied") : t("common.copy")}
          </button>
        </div>
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
