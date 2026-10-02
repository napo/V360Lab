/** Remaining time from the time spent so far, or null before the first item. */
export function remainingMs(elapsedMs: number, done: number, total: number): number | null {
  if (done <= 0 || total <= done) return done >= total && total > 0 ? 0 : null;
  return (elapsedMs / done) * (total - done);
}

/** "45 s", "3 min", "1 h 20 min": rounded for people, not machines. */
export function formatDurationRough(ms: number): string {
  const seconds = Math.max(1, Math.round(ms / 1000));
  if (seconds < 90) return `${seconds} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 90) return `${minutes} min`;
  return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}
