import { useI18n } from "../hooks/useI18n";

/** The V360Lab logo, in the variant matching the color scheme. */
export function BrandHero() {
  const { t } = useI18n();
  return (
    <div className="brand-hero">
      <img className="logo-light" src="/brand/logo-light-bg.png" alt={t("app.name")} />
      <img className="logo-dark" src="/brand/logo-dark-bg.png" alt={t("app.name")} />
    </div>
  );
}
