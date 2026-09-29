import { useContext } from "react";
import { DownloadsContext, type DownloadsContextValue } from "../context/DownloadsContext";

export function useDownloads(): DownloadsContextValue {
  const context = useContext(DownloadsContext);
  if (!context) throw new Error("useDownloads must be used inside <DownloadsProvider>");
  return context;
}
