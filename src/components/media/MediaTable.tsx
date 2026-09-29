import { useI18n } from "../../hooks/useI18n";
import type { TranslationKey } from "../../i18n/types";
import type { MediaItem } from "../../types/camera";
import type { DownloadOptions } from "../../types/downloads";
import { MediaRow } from "./MediaRow";

const COLUMNS: TranslationKey[] = [
  "media.colPreview",
  "media.colFile",
  "media.colDate",
  "media.colDuration",
  "media.colSize",
  "media.colLens",
  "media.colTelemetry",
  "media.colActions",
];

export function MediaTable({ items, options }: { items: MediaItem[]; options: DownloadOptions }) {
  const { t } = useI18n();
  return (
    <div className="table-wrapper">
      <table className="data-table media-table">
        <thead>
          <tr>
            {COLUMNS.map((key) => (
              <th key={key}>{t(key)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <MediaRow key={item.id} item={item} options={options} />
          ))}
        </tbody>
      </table>
    </div>
  );
}
