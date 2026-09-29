import { useEffect, useRef, useState } from "react";
import { useThumbnail } from "../../hooks/useThumbnail";
import type { MediaType } from "../../types/camera";

/** Loads the thumbnail only once the row scrolls into view: a full card
 * lists hundreds of files and the camera's HTTP server is slow. */
export function Thumbnail({ url, mediaType }: { url: string | null; mediaType: MediaType }) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!element || visible) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) setVisible(true);
      },
      { rootMargin: "200px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [visible]);

  const { src, failed } = useThumbnail(visible ? url : null);
  const label = mediaType === "video" ? "VIDEO" : mediaType === "photo" ? "PHOTO" : "FILE";

  return (
    <div ref={ref}>
      {src ? (
        <img className="thumbnail" src={src} alt="" />
      ) : (
        <div className="thumbnail thumbnail-placeholder">{url && visible && !failed ? "…" : label}</div>
      )}
    </div>
  );
}
