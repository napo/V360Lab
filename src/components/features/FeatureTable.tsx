import type { CameraFeature, JsonValue } from "../../types/camera";
import { formatJsonValue } from "../../utils/format";

function sameValue(a: JsonValue, b: JsonValue): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function FeatureTable({ features }: { features: CameraFeature[] }) {
  return (
    <div className="table-wrapper">
      <table className="data-table">
        <thead>
          <tr>
            <th>Feature</th>
            <th>Current value</th>
            <th>Available options</th>
            <th>Enabled</th>
          </tr>
        </thead>
        <tbody>
          {features.map((feature) => (
            <tr key={feature.key}>
              <td>
                <div>{feature.label ?? feature.key}</div>
                <div className="mono muted small">{feature.key}</div>
              </td>
              <td className="mono">{formatJsonValue(feature.value)}</td>
              <td>
                {feature.options.length === 0 ? (
                  <span className="muted">—</span>
                ) : (
                  <ul className="option-list">
                    {feature.options.map((option, index) => (
                      <li
                        key={JSON.stringify(option)}
                        className={sameValue(option, feature.value) ? "current" : undefined}
                        title={feature.optionSummaries[index]}
                      >
                        <span className="mono">{formatJsonValue(option)}</span>
                        {feature.optionSummaries[index] && (
                          <span className="muted small"> — {feature.optionSummaries[index]}</span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </td>
              <td>
                {feature.enabled == null ? (
                  <span className="muted">—</span>
                ) : (
                  <span className={`tag ${feature.enabled ? "tag-on" : "tag-off"}`}>
                    {feature.enabled ? "yes" : "no"}
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
