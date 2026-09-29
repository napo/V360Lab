import { useEffect, useState } from "react";
import { loadThumbnail } from "../services/thumbnailService";

interface ThumbnailState {
  src: string | null;
  failed: boolean;
}

/** Loads a camera thumbnail through the backend (as a data URL). */
export function useThumbnail(url: string | null): ThumbnailState {
  const [state, setState] = useState<ThumbnailState>({ src: null, failed: false });

  useEffect(() => {
    if (!url) return;
    let active = true;
    setState({ src: null, failed: false });
    loadThumbnail(url).then(
      (src) => active && setState({ src, failed: false }),
      () => active && setState({ src: null, failed: true }),
    );
    return () => {
      active = false;
    };
  }, [url]);

  return url ? state : { src: null, failed: false };
}
