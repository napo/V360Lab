import type { MediaItem } from "../../types/camera";
import type { DownloadOptions } from "../../types/downloads";
import { MediaCard } from "./MediaCard";

interface MediaGridProps {
  items: MediaItem[];
  options: DownloadOptions;
  selected: Set<string>;
  onToggleSelected: (item: MediaItem) => void;
  onDelete: (item: MediaItem) => void;
  deleteDisabled: boolean;
}

export function MediaGrid({ items, options, selected, onToggleSelected, onDelete, deleteDisabled }: MediaGridProps) {
  return (
    <div className="media-grid">
      {items.map((item) => (
        <MediaCard
          key={item.id}
          item={item}
          options={options}
          selected={selected.has(item.id)}
          onToggleSelected={onToggleSelected}
          onDelete={onDelete}
          deleteDisabled={deleteDisabled}
        />
      ))}
    </div>
  );
}
