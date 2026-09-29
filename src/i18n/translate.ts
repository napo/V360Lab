import { Fragment, createElement, type ReactNode } from "react";
import type { AppError } from "../types/errors";
import { en } from "./en";
import { it } from "./it";
import type { Dictionary, Language, TranslateParams, TranslationKey } from "./types";

export const DICTIONARIES: Record<Language, Dictionary> = { en, it };

/** Languages offered in the UI, named in their own language. */
export const LANGUAGES: Array<{ code: Language; name: string }> = [
  { code: "en", name: "English" },
  { code: "it", name: "Italiano" },
];

export function isLanguage(value: unknown): value is Language {
  return value === "en" || value === "it";
}

/** Best match for the system/browser languages; English otherwise. */
export function detectLanguage(preferred?: readonly string[]): Language {
  const candidates =
    preferred ?? (typeof navigator === "undefined" ? [] : navigator.languages ?? [navigator.language]);
  for (const tag of candidates) {
    const base = tag?.toLowerCase().split("-")[0];
    if (isLanguage(base)) return base;
  }
  return "en";
}

export function hasKey(key: string): key is TranslationKey {
  return Object.prototype.hasOwnProperty.call(en, key);
}

/** Replaces `{name}` placeholders; unknown placeholders are left as is. */
export function interpolate(template: string, params?: TranslateParams): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = params?.[name];
    return value === undefined || value === null ? match : String(value);
  });
}

/** Resolves a key (or plural base via `params.count`) to its template. */
function template(language: Language, key: string, params?: TranslateParams): string {
  const dictionary = DICTIONARIES[language] as Record<string, string>;
  const fallback = en as Record<string, string>;
  let resolved = key;
  // Plural keys (`_one`, `_other`, …) are used only when they exist.
  if (typeof params?.count === "number" && `${key}_other` in fallback) {
    const form = new Intl.PluralRules(language).select(params.count);
    resolved = `${key}_${form}` in fallback ? `${key}_${form}` : `${key}_other`;
  }
  return dictionary[resolved] ?? fallback[resolved] ?? key;
}

export function translate(language: Language, key: string, params?: TranslateParams): string {
  return interpolate(template(language, key, params), params);
}

/** Like `translate`, but placeholders may be React nodes (e.g. <code>). */
export function translateNodes(
  language: Language,
  key: string,
  params: Record<string, ReactNode>,
): ReactNode {
  const parts = template(language, key, params as TranslateParams).split(/(\{\w+\})/g);
  return createElement(
    Fragment,
    null,
    ...parts.map((part, index) => {
      const name = /^\{(\w+)\}$/.exec(part)?.[1];
      const value = name !== undefined && name in params ? params[name] : part;
      return createElement(Fragment, { key: index }, value);
    }),
  );
}

/**
 * User-facing message for a backend error: `errors.<kind>`, refined by
 * `params.resource` / `params.reason` when such a key exists. Falls back to
 * the English message produced by the backend.
 */
export function describeError(language: Language, error: AppError): string {
  const params = error.params ?? {};
  const refinements = [params.resource, params.reason].filter(
    (value): value is string => typeof value === "string",
  );
  const candidates = [...refinements.map((r) => `errors.${error.kind}.${r}`), `errors.${error.kind}`];
  const key = candidates.find(hasKey);
  return key ? translate(language, key, params) : error.message;
}
