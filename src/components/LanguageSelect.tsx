import { useI18n } from "../hooks/useI18n";
import { LANGUAGES } from "../i18n/translate";
import { isLanguage } from "../i18n/translate";

interface LanguageSelectProps {
  /** Offer "system default" as a choice (settings page). */
  includeSystem?: boolean;
  className?: string;
}

/** Changes the UI language immediately and persists the choice. */
export function LanguageSelect({ includeSystem = false, className }: LanguageSelectProps) {
  const { t, language, preference, systemLanguage, setLanguage } = useI18n();
  const systemName = LANGUAGES.find((l) => l.code === systemLanguage)?.name ?? systemLanguage;
  const value = includeSystem ? (preference ?? "system") : language;

  return (
    <select
      className={className}
      aria-label={t("sidebar.language")}
      value={value}
      onChange={(e) => void setLanguage(isLanguage(e.target.value) ? e.target.value : null)}
    >
      {includeSystem && (
        <option value="system">{t("settings.languageSystem", { language: systemName })}</option>
      )}
      {LANGUAGES.map((l) => (
        <option key={l.code} value={l.code}>
          {l.name}
        </option>
      ))}
    </select>
  );
}
