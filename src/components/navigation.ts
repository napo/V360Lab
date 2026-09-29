import type { TranslationKey } from "../i18n/types";

export type PageId = "connect" | "capture" | "media" | "advanced";

export interface PageDefinition {
  id: PageId;
  labelKey: TranslationKey;
  /** Pages that need a connected camera. */
  requiresCamera: boolean;
}

/** The four sections of the app, in navigation order. */
export const PAGES: PageDefinition[] = [
  { id: "connect", labelKey: "nav.connect", requiresCamera: false },
  { id: "capture", labelKey: "nav.capture", requiresCamera: true },
  { id: "media", labelKey: "nav.media", requiresCamera: true },
  { id: "advanced", labelKey: "nav.advanced", requiresCamera: false },
];

export type Navigate = (page: PageId) => void;
