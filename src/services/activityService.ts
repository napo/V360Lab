import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ActivityEvent } from "../types/activity";
import { isTauri } from "./backend";

export const activityService = {
  /** Subscribes to backend activity steps; returns an unsubscribe function. */
  onStep: async (handler: (event: ActivityEvent) => void): Promise<UnlistenFn> => {
    if (!isTauri()) return () => {};
    return listen<ActivityEvent>("activity", (event) => handler(event.payload));
  },
};
