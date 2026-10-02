import { ask, open } from "@tauri-apps/plugin-dialog";
import type { AppInfo, Settings, SettingsView } from "../types/settings";
import { call } from "./backend";

export const settingsService = {
  get: () => call<SettingsView>("get_settings"),
  update: (settings: Settings) => call<SettingsView>("update_settings", { settings }),
  appInfo: () => call<AppInfo>("app_info"),

  /** Native confirmation dialog for destructive actions. */
  confirm: (message: string, options: { title: string; okLabel: string; cancelLabel: string }) =>
    ask(message, { ...options, kind: "warning" }),

  /** Native file picker for an ONNX model; resolves to null when cancelled. */
  pickModel: async (title: string): Promise<string | null> => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "ONNX", extensions: ["onnx"] }],
      title,
    });
    return typeof selected === "string" ? selected : null;
  },

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
