import { useEffect, useState } from "react";
import { useI18n } from "../hooks/useI18n";
import type { ActivityState } from "../hooks/useActivity";
import { ErrorBanner } from "./ErrorBanner";
import { Meter } from "./Meter";
import { Spinner } from "./Spinner";

interface ActivityPanelProps {
  activity: ActivityState;
  title: string;
  /** When finished successfully, show only a one-line summary. */
  collapseWhenDone?: boolean;
}

/** Live feedback for a slow operation: spinner, elapsed time and steps. */
export function ActivityPanel({ activity, title, collapseWhenDone = false }: ActivityPanelProps) {
  const { t, lookup } = useI18n();
  const [now, setNow] = useState(Date.now());
  const [expanded, setExpanded] = useState(false);
  const running = activity.status === "running";

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => setNow(Date.now()), 200);
    return () => window.clearInterval(id);
  }, [running]);

  useEffect(() => setExpanded(false), [activity.id]);

  if (activity.status === "idle" || activity.startedAt === null) return null;

  const end = activity.finishedAt ?? now;
  const seconds = Math.max(0, (end - activity.startedAt) / 1000).toFixed(1);
  const lastStep = activity.steps[activity.steps.length - 1];
  const collapsed = collapseWhenDone && activity.status === "done" && !expanded;

  const message = (code: string, params: Record<string, unknown>) =>
    lookup(`activity.${code}`, params) ?? code;

  return (
    <section className={`activity activity-${activity.status}`} aria-live="polite">
      <header className="activity-header">
        {running ? (
          <Spinner size={26} />
        ) : (
          <span className="activity-icon">{activity.status === "done" ? "✓" : "✕"}</span>
        )}
        <strong>{title}</strong>
        <span className="activity-time mono">{t("activity.elapsed", { seconds })}</span>
        {collapseWhenDone && activity.status === "done" && (
          <button type="button" className="btn btn-small btn-ghost" onClick={() => setExpanded((v) => !v)}>
            {expanded ? t("media.hide") : t("media.details")}
          </button>
        )}
      </header>

      {collapsed && lastStep && <p className="activity-summary">{message(lastStep.code, lastStep.params)}</p>}

      {!collapsed && (
        <ol className="activity-steps">
          {activity.steps.map((step, index) => {
            const current = running && index === activity.steps.length - 1;
            const icon = current ? null : step.level === "warning" ? "!" : "✓";
            return (
              <li key={step.key} className={`activity-step level-${step.level} ${current ? "current" : ""}`}>
                <span className="activity-step-icon">{current ? <Spinner size={14} /> : icon}</span>
                <span>{message(step.code, step.params)}</span>
                {current && step.progress && (
                  <Meter fraction={step.progress.total ? step.progress.done / step.progress.total : null} />
                )}
              </li>
            );
          })}
        </ol>
      )}

      {activity.status === "failed" && activity.error && (
        <ErrorBanner error={activity.error} title={t("activity.failed")} />
      )}
    </section>
  );
}
