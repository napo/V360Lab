export type PageId = "dashboard" | "media" | "features" | "settings" | "about";

export interface PageDefinition {
  id: PageId;
  label: string;
  /** Pages that show the connection screen when no camera is connected. */
  requiresCamera: boolean;
}

export const PAGES: PageDefinition[] = [
  { id: "dashboard", label: "Dashboard", requiresCamera: true },
  { id: "media", label: "Media", requiresCamera: true },
  { id: "features", label: "Camera Features", requiresCamera: true },
  { id: "settings", label: "Settings", requiresCamera: false },
  { id: "about", label: "About", requiresCamera: false },
];
