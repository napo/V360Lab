import { createContext, useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { settingsService } from "../services/settingsService";
import type { AppError } from "../types/errors";
import type { Settings, SettingsView } from "../types/settings";
import { toAppError } from "../utils/errors";

export interface SettingsContextValue {
  view: SettingsView | null;
  loading: boolean;
  error: AppError | null;
  debugMode: boolean;
  /** Persists settings; rejects with an AppError on failure. */
  save: (settings: Settings) => Promise<void>;
  reload: () => Promise<void>;
}

export const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<SettingsView | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<AppError | null>(null);

  const reload = useCallback(async () => {
    try {
      setView(await settingsService.get());
      setError(null);
    } catch (e) {
      setError(toAppError(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const save = useCallback(async (settings: Settings) => {
    setView(await settingsService.update(settings));
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const value = useMemo(
    () => ({
      view,
      loading,
      error,
      debugMode: view?.settings.debugMode ?? false,
      save,
      reload,
    }),
    [view, loading, error, save, reload],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}
