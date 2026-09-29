import type { en } from "./en";

export type Language = "en" | "it";

export type TranslationKey = keyof typeof en;

/** Every language provides exactly the keys of the English dictionary. */
export type Dictionary = Record<TranslationKey, string>;

type PluralBase<K> = K extends `${infer Base}_one` ? Base : never;

/** Base names of plural keys, e.g. `media.count` for `media.count_one`. */
export type PluralKey = PluralBase<TranslationKey>;

export type TranslateParams = Record<string, unknown>;
