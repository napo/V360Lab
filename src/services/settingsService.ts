import { open } from "@tauri-apps/plugin-dialog";
import type { AppInfo, Settings, SettingsView } from "../types/settings";
import { call } from "./backend";

export const settingsService = {
  get: () => call<SettingsView>("get_settings"),
  update: (settings: Settings) => call<SettingsView>("update_settings", { settings }),
  appInfo: () => call<AppInfo>("app_info"),

  /** Native folder picker; resolves to null when cancelled. */
  pickDirectory: async (current: string | null, title: string): Promise<string | null> => {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: current ?? undefined,
      title,
    });
    return typeof selected === "string" ? selected : null;
  },
};
