import type { JsonValue } from "../types/camera";

export const EMPTY = "—";

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return EMPTY;
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

/** Seconds to `m:ss` or `h:mm:ss`. */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return EMPTY;
  const total = Math.round(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** ISO date or Unix seconds to a local `YYYY-MM-DD HH:MM:SS` string. */
export function formatDateTime(value: string | number | null | undefined): string {
  if (value == null) return EMPTY;
  const date = typeof value === "number" ? new Date(value * 1000) : new Date(value);
  if (Number.isNaN(date.getTime())) return EMPTY;
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ` +
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  );
}

export function formatPercent(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return EMPTY;
  return `${Math.round(value)}%`;
}

export function formatCoordinate(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return EMPTY;
  return value.toFixed(6);
}

/** Renders any JSON value compactly for tables. */
export function formatJsonValue(value: JsonValue | undefined): string {
  if (value === null || value === undefined || value === "") return EMPTY;
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

/** Fraction in [0, 1] of used storage, or null if unknown. */
export function storageUsedFraction(
  total: number | null | undefined,
  available: number | null | undefined,
): number | null {
  if (!total || total <= 0 || available == null || available < 0) return null;
  return Math.min(1, Math.max(0, (total - available) / total));
}
