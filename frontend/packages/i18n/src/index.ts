import type { Catalog } from './catalog.js';
import { en } from './locales/en/index.js';

export { pluralizer, type Catalog } from './catalog.js';
export { en };

export type Messages = Catalog<typeof en>;

/** Languages the interface ships with. */
export const locales = ['en'] as const;
export type Locale = (typeof locales)[number];

export const defaultLocale: Locale = 'en';

const loaders: Record<Locale, () => Promise<Messages>> = {
	en: async () => en
};

export function loadMessages(locale: Locale): Promise<Messages> {
	return loaders[locale]();
}

export function isLocale(value: string): value is Locale {
	return (locales as readonly string[]).includes(value);
}
