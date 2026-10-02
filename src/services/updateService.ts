import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { UpdateInfo } from "../types/settings";
import { call } from "./backend";

const LAST_CHECK_KEY = "v360lab.update.lastCheck";
const SKIPPED_KEY = "v360lab.update.skipped";
/** Restarting the app during development must not ask GitHub every time. */
const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Preference only.
  }
}

/** Lines of the "What's new" part of the release notes, without Markdown. */
export function releaseHighlights(notes: string, max = 6): string[] {
  const lines = notes.split("\n");
  const start = lines.findIndex((l) => /^##\s+what's new/i.test(l.trim()));
  const section = start >= 0 ? lines.slice(start + 1) : lines;
  const end = section.findIndex((l) => /^##\s/.test(l.trim()));
  return (end >= 0 ? section.slice(0, end) : section)
    .map((l) => l.trim())
    .filter((l) => l.startsWith("- "))
    .map((l) => l.slice(2).replace(/\*\*(.+?)\*\*/g, "$1").replace(/`([^`]+)`/g, "$1"))
    .slice(0, max);
}

export const updateService = {
  check: () => call<UpdateInfo>("check_for_update"),

  /** True when the automatic check at start-up should run now. */
  dueAtStartup: (now = Date.now()) => {
    const last = Number(read(LAST_CHECK_KEY) ?? 0);
    return !Number.isFinite(last) || now - last > CHECK_INTERVAL_MS;
  },
  markChecked: (now = Date.now()) => write(LAST_CHECK_KEY, String(now)),
  isSkipped: (version: string) => read(SKIPPED_KEY) === version,
  skip: (version: string) => write(SKIPPED_KEY, version),

  /**
   * Desktop: downloads the signed update, installs it and restarts the
   * app. Resolves to false when no signed update is offered for this
   * installation (e.g. a .deb or .rpm package), so the UI can open the
   * download page instead.
   */
  installDesktop: async (onProgress: (received: number, total: number | null) => void) => {
    const update = await check();
    if (!update) return false;
    let received = 0;
    let total: number | null = null;
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") total = event.data.contentLength ?? null;
      if (event.event === "Progress") {
        received += event.data.chunkLength;
        onProgress(received, total);
      }
    });
    await relaunch();
    return true;
  },

  open: (url: string) => openUrl(url),
};
