import {
	defaultLocale,
	en,
	isLocale,
	loadMessages,
	locales,
	type Locale,
	type Messages
} from '@lorealm/i18n';

let locale = $state<Locale>(defaultLocale);
let messages = $state.raw<Messages>(en);

/** The active interface messages. Reads are reactive. */
export const t: Messages = new Proxy({} as Messages, {
	get: (_, key) => messages[key as keyof Messages]
});

/** A lookup table that follows the active locale. */
export function localized<T extends object>(pick: (messages: Messages) => T): T {
	const current = () => pick(messages);
	return new Proxy({} as T, {
		get: (_, key) => Reflect.get(current(), key),
		has: (_, key) => Reflect.has(current(), key),
		ownKeys: () => Reflect.ownKeys(current()),
		getOwnPropertyDescriptor: (_, key) => {
			const descriptor = Reflect.getOwnPropertyDescriptor(current(), key);
			return descriptor && { ...descriptor, configurable: true };
		}
	});
}

export function currentLocale(): Locale {
	return locale;
}

let pending = 0;

export async function setLocale(next: Locale): Promise<void> {
	const call = ++pending;
	const loaded = await loadMessages(next);
	if (call !== pending) return; // A later choice won.
	locale = next;
	messages = loaded;
	if (typeof document !== 'undefined') document.documentElement.lang = next;
}

/** The shipped locale closest to the device's preferred languages. */
export function deviceLocale(): Locale {
	const wanted = typeof navigator === 'undefined' ? [] : navigator.languages;
	const lower = locales.map((l) => l.toLowerCase());
	for (const tag of wanted) {
		const exact = lower.indexOf(tag.toLowerCase());
		if (exact !== -1) return locales[exact];
		const base = tag.split('-')[0].toLowerCase();
		const loose = lower.findIndex((l) => l.split('-')[0] === base);
		if (loose !== -1) return locales[loose];
	}
	return defaultLocale;
}

export function resolveLocale(choice: string | null): Locale {
	return choice !== null && isLocale(choice) ? choice : deviceLocale();
}

/** A locale's name in its own language, e.g. "Deutsch". */
export function localeName(tag: Locale): string {
	try {
		const name = new Intl.DisplayNames([tag], { type: 'language' }).of(tag) ?? tag;
		return name.charAt(0).toLocaleUpperCase(tag) + name.slice(1);
	} catch {
		return tag;
	}
}
