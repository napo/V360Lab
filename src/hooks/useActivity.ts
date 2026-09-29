import { useCallback, useEffect, useRef, useState } from "react";
import { activityService } from "../services/activityService";
import type { ActivityLevel, ActivityStep } from "../types/activity";
import type { AppError } from "../types/errors";

export type ActivityStatus = "idle" | "running" | "done" | "failed";

export interface ActivityEntry extends ActivityStep {
  key: number;
}

export interface ActivityState {
  id: string | null;
  status: ActivityStatus;
  steps: ActivityEntry[];
  startedAt: number | null;
  finishedAt: number | null;
  error: AppError | null;
}

const IDLE: ActivityState = {
  id: null,
  status: "idle",
  steps: [],
  startedAt: null,
  finishedAt: null,
  error: null,
};

function newId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `activity-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export interface Activity {
  state: ActivityState;
  /** Starts a new activity and returns its id (pass it to the backend). */
  start: () => string;
  /** Adds a step produced by the frontend itself. */
  step: (code: string, params?: Record<string, unknown>, level?: ActivityLevel) => void;
  finish: () => void;
  fail: (error: AppError) => void;
  reset: () => void;
}

/**
 * Tracks one slow operation as a list of steps. Steps come from the backend
 * (`activity` events tagged with this activity's id) and from the frontend.
 * Consecutive informational steps with the same code replace each other, so
 * a scan shows one advancing line instead of dozens.
 */
export function useActivity(): Activity {
  const [state, setState] = useState<ActivityState>(IDLE);
  const currentId = useRef<string | null>(null);
  const counter = useRef(0);

  const append = useCallback((step: ActivityStep) => {
    setState((current) => {
      if (current.status !== "running") return current;
      const entry = { ...step, key: ++counter.current };
      const last = current.steps[current.steps.length - 1];
      // A repeated informational step (scan progress, "probing host …")
      // supersedes the previous one instead of growing the list.
      const replace = last && last.code === step.code && last.level === "info" && step.level === "info";
      const steps = replace ? [...current.steps.slice(0, -1), entry] : [...current.steps, entry];
      return { ...current, steps };
    });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    void activityService
      .onStep((event) => {
        if (event.activityId !== currentId.current) return;
        const { activityId: _ignored, ...step } = event;
        append(step);
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [append]);

  const start = useCallback(() => {
    const id = newId();
    currentId.current = id;
    setState({ ...IDLE, id, status: "running", startedAt: Date.now() });
    return id;
  }, []);

  const step = useCallback(
    (code: string, params: Record<string, unknown> = {}, level: ActivityLevel = "info") =>
      append({ code, params, level, progress: null }),
    [append],
  );

  const finish = useCallback(
    () =>
      setState((current) =>
        current.status === "running" ? { ...current, status: "done", finishedAt: Date.now() } : current,
      ),
    [],
  );

  const fail = useCallback(
    (error: AppError) =>
      setState((current) => ({ ...current, status: "failed", error, finishedAt: Date.now() })),
    [],
  );

  const reset = useCallback(() => {
    currentId.current = null;
    setState(IDLE);
  }, []);

  return { state, start, step, finish, fail, reset };
}
