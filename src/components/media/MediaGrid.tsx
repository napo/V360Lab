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
  /** Absent when the camera cannot mark favourites. */
  onToggleFavorite?: (item: MediaItem) => void;
  favoriteBusy: string | null;
  onOpen: (item: MediaItem) => void;
}

export function MediaGrid({
  items,
  options,
  selected,
  onToggleSelected,
  onDelete,
  deleteDisabled,
  onToggleFavorite,
  favoriteBusy,
  onOpen,
}: MediaGridProps) {
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
          onToggleFavorite={onToggleFavorite}
          favoriteBusy={favoriteBusy === item.id}
          onOpen={onOpen}
        />
      ))}
    </div>
  );
}
