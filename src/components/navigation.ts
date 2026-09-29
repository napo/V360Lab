import type { TranslationKey } from "../i18n/types";

export type PageId = "dashboard" | "media" | "features" | "settings" | "about";

export interface PageDefinition {
  id: PageId;
  labelKey: TranslationKey;
  /** Pages that show the connection screen when no camera is connected. */
  requiresCamera: boolean;
}

export const PAGES: PageDefinition[] = [
  { id: "dashboard", labelKey: "nav.dashboard", requiresCamera: true },
  { id: "media", labelKey: "nav.media", requiresCamera: true },
  { id: "features", labelKey: "nav.features", requiresCamera: true },
  { id: "settings", labelKey: "nav.settings", requiresCamera: false },
  { id: "about", labelKey: "nav.about", requiresCamera: false },
];
