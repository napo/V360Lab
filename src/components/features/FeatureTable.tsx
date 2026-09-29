import { useI18n } from "../../hooks/useI18n";
import { featureLabel } from "../../i18n/featureLabels";
import type { CameraFeature } from "../../types/camera";
import { FeatureControl } from "./FeatureControl";

export function FeatureTable({ features }: { features: CameraFeature[] }) {
  const { t, lookup } = useI18n();
  return (
    <div className="table-wrapper">
      <table className="data-table">
        <thead>
          <tr>
            <th>{t("features.colFeature")}</th>
            <th>{t("features.colValue")}</th>
            <th>{t("features.colEnabled")}</th>
          </tr>
        </thead>
        <tbody>
          {features.map((feature) => (
            <tr key={feature.key}>
              <td>
                <div>{featureLabel(lookup, feature)}</div>
                <div className="mono muted small">{feature.key}</div>
              </td>
              <td>
                <FeatureControl feature={feature} />
              </td>
              <td>
                {feature.enabled == null ? (
                  <span className="muted">—</span>
                ) : (
                  <span className={`tag ${feature.enabled ? "tag-on" : "tag-off"}`}>
                    {feature.enabled ? t("common.yes") : t("common.no")}
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
