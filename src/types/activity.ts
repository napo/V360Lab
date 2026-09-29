// Mirrors src-tauri/src/activity.rs.

export type ActivityLevel = "info" | "success" | "warning";

export interface ActivityStep {
  /** Stable code, translated as `activity.<code>`. */
  code: string;
  level: ActivityLevel;
  params: Record<string, unknown>;
  progress: { done: number; total: number } | null;
}

export interface ActivityEvent extends ActivityStep {
  activityId: string;
}
