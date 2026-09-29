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

interface MediaTableProps {
  items: MediaItem[];
  options: DownloadOptions;
  selected: Set<string>;
  onToggleSelected: (item: MediaItem) => void;
  onToggleAll: () => void;
  onDelete: (item: MediaItem) => void;
  deleteDisabled: boolean;
}

export function MediaTable({
  items,
  options,
  selected,
  onToggleSelected,
  onToggleAll,
  onDelete,
  deleteDisabled,
}: MediaTableProps) {
  const { t } = useI18n();
  const allSelected = items.length > 0 && items.every((item) => selected.has(item.id));

  return (
    <div className="table-wrapper">
      <table className="data-table media-table">
        <thead>
          <tr>
            <th className="checkbox-cell">
              <input
                type="checkbox"
                checked={allSelected}
                aria-label={t("media.selectAll")}
                onChange={onToggleAll}
              />
            </th>
            {COLUMNS.map((key) => (
              <th key={key}>{t(key)}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <MediaRow
              key={item.id}
              item={item}
              options={options}
              selected={selected.has(item.id)}
              onToggleSelected={onToggleSelected}
              onDelete={onDelete}
              deleteDisabled={deleteDisabled}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
