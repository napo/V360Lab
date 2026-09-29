import { createContext, useCallback, useEffect, useMemo, type ReactNode } from "react";
import { useSettings } from "../hooks/useSettings";
import type { AppError } from "../types/errors";
import {
  describeError,
  detectLanguage,
  hasKey,
  isLanguage,
  translate,
  translateNodes,
} from "./translate";
import type { Language, PluralKey, TranslateParams, TranslationKey } from "./types";

export interface I18nContextValue {
  /** Language in use. */
  language: Language;
  /** Explicit user choice; null follows the system language. */
  preference: Language | null;
  systemLanguage: Language;
  t: (key: TranslationKey | PluralKey, params?: TranslateParams) => string;
  /** Translation whose placeholders can be React nodes. */
  tx: (key: TranslationKey | PluralKey, params: Record<string, ReactNode>) => ReactNode;
  errorMessage: (error: AppError) => string;
  /** Translation for a computed key, or null when the key does not exist. */
  lookup: (key: string, params?: TranslateParams) => string | null;
  /** Persists the choice in the settings (null = system language). */
  setLanguage: (language: Language | null) => Promise<void>;
}

export const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const { view, save } = useSettings();
  const systemLanguage = useMemo(() => detectLanguage(), []);
  const stored = view?.settings.language;
  const preference = isLanguage(stored) ? stored : null;
  const language = preference ?? systemLanguage;

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const setLanguage = useCallback(
    async (next: Language | null) => {
      if (view) await save({ ...view.settings, language: next });
    },
    [view, save],
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      language,
      preference,
      systemLanguage,
      t: (key, params) => translate(language, key, params),
      tx: (key, params) => translateNodes(language, key, params),
      errorMessage: (error) => describeError(language, error),
      lookup: (key, params) =>
        hasKey(key) || hasKey(`${key}_other`) ? translate(language, key, params) : null,
      setLanguage,
    }),
    [language, preference, systemLanguage, setLanguage],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
