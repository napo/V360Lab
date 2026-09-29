import type { MediaItem } from "../../types/camera";
import type { DownloadOptions } from "../../types/downloads";
import { MediaRow } from "./MediaRow";

export function MediaTable({ items, options }: { items: MediaItem[]; options: DownloadOptions }) {
  return (
    <div className="table-wrapper">
      <table className="data-table media-table">
        <thead>
          <tr>
            <th>Preview</th>
            <th>File</th>
            <th>Date / time</th>
            <th>Duration</th>
            <th>Size</th>
            <th>Lens</th>
            <th>Telemetry</th>
            <th>Actions</th>
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
