import { IntlMessageFormat } from 'intl-messageformat';
import { locales, source, translations, type Locale } from './generated/locales.js';
import type { Messages } from './generated/messages.js';
import type { Tree } from './tree.js';

export { locales, type Locale, type Messages };

export const defaultLocale: Locale = 'en';

type Args = Record<string, string | number | boolean | Date>;
type Compiled = { [key: string]: string | ((args: Args) => string) | Compiled };

const options = { ignoreTag: true };

/** True when the text has no ICU syntax. */
const isLiteral = (text: string) => !/[{}]/.test(text);

function format(text: string, locale: string, args?: Args): string {
	return new IntlMessageFormat(text, locale, undefined, options).format(args) as string;
}

/**
 * One message in `locale`. Falls back to English if it's missing.
 */
function message(
	english: string,
	translated: string | undefined,
	locale: string
): Compiled[string] {
	const ast = isLiteral(english)
		? []
		: new IntlMessageFormat(english, 'en', undefined, options).getAst();
	const takesArgs = ast.some((el) => el.type !== 0);

	if (!takesArgs) {
		if (translated !== undefined) {
			try {
				return isLiteral(translated) ? translated.replace(/''/g, "'") : format(translated, locale);
			} catch {
				// Falls through to English.
			}
		}
		return isLiteral(english) ? english.replace(/''/g, "'") : format(english, 'en');
	}

	let formatter: IntlMessageFormat | undefined;
	let fallback: IntlMessageFormat | undefined;
	const englishFormatter = () =>
		(fallback ??= new IntlMessageFormat(english, 'en', undefined, options));
	return (args: Args) => {
		if (translated !== undefined) {
			try {
				formatter ??= new IntlMessageFormat(translated, locale, undefined, options);
				return formatter.format(args) as string;
			} catch {
				translated = undefined;
			}
		}
		return englishFormatter().format(args) as string;
	};
}

function compile(english: Tree, translated: unknown, locale: string): Compiled {
	const tr =
		typeof translated === 'object' && translated !== null
			? (translated as Record<string, unknown>)
			: {};
	const out: Compiled = {};
	for (const [key, value] of Object.entries(english)) {
		out[key] =
			typeof value === 'string'
				? message(value, typeof tr[key] === 'string' ? tr[key] : undefined, locale)
				: compile(value, tr[key], locale);
	}
	return out;
}

export const en = compile(source, undefined, 'en') as unknown as Messages;

const loaded = new Map<Locale, Promise<Messages>>([['en', Promise.resolve(en)]]);

export function loadMessages(locale: Locale): Promise<Messages> {
	let messages = loaded.get(locale);
	if (!messages) {
		const load = translations[locale];
		messages = (load ? load().catch(() => ({})) : Promise.resolve({})).then(
			(files) => compile(source, files, locale) as unknown as Messages
		);
		loaded.set(locale, messages);
	}
	return messages;
}

export function isLocale(value: string): value is Locale {
	return (locales as readonly string[]).includes(value);
}
