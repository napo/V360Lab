import { useContext } from "react";
import { CameraContext, type CameraContextValue } from "../context/CameraContext";

export function useCamera(): CameraContextValue {
  const context = useContext(CameraContext);
  if (!context) throw new Error("useCamera must be used inside <CameraProvider>");
  return context;
}
