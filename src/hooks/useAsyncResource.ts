import { useCallback, useEffect, useRef, useState } from "react";
import type { AppError } from "../types/errors";
import { toAppError } from "../utils/errors";

export interface AsyncResource<T> {
  data: T | null;
  error: AppError | null;
  loading: boolean;
  reload: () => Promise<void>;
}

/**
 * Loads data when `enabled` becomes true and on `reload()`. Previous data is
 * kept while reloading and when a reload fails; stale responses are ignored.
 */
export function useAsyncResource<T>(loader: () => Promise<T>, enabled = true): AsyncResource<T> {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(enabled);
  const loaderRef = useRef(loader);
  const requestId = useRef(0);

  useEffect(() => {
    loaderRef.current = loader;
  });

  const reload = useCallback(async () => {
    const id = ++requestId.current;
    setLoading(true);
    setError(null);
    try {
      const result = await loaderRef.current();
      if (id === requestId.current) setData(result);
    } catch (e) {
      if (id === requestId.current) setError(toAppError(e));
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (enabled) void reload();
  }, [enabled, reload]);

  return { data, error, loading, reload };
}
