import { useEffect, useState } from "react";

function isVisible(): boolean {
  return typeof document === "undefined" || document.visibilityState !== "hidden";
}

/**
 * False while the app is in the background or the screen is off, so that
 * work nobody sees (live preview, frequent polling) can pause.
 */
export function usePageVisible(): boolean {
  const [visible, setVisible] = useState(isVisible);
  useEffect(() => {
    const update = () => setVisible(isVisible());
    document.addEventListener("visibilitychange", update);
    return () => document.removeEventListener("visibilitychange", update);
  }, []);
  return visible;
}
