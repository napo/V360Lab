import { useThumbnail } from "../../hooks/useThumbnail";
import type { MediaType } from "../../types/camera";

export function Thumbnail({ url, mediaType }: { url: string | null; mediaType: MediaType }) {
  const { src, failed } = useThumbnail(url);
  if (src) return <img className="thumbnail" src={src} alt="" loading="lazy" />;
  return (
    <div className="thumbnail thumbnail-placeholder">
      {url && !failed ? "…" : mediaType === "video" ? "VIDEO" : mediaType === "photo" ? "PHOTO" : "FILE"}
    </div>
  );
}
