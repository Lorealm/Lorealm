import { defaultLocale, en, loadMessages, type Locale, type Messages } from '@lorealm/i18n';

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

export async function setLocale(next: Locale): Promise<void> {
	const loaded = await loadMessages(next);
	locale = next;
	messages = loaded;
	if (typeof document !== 'undefined') document.documentElement.lang = next;
}
