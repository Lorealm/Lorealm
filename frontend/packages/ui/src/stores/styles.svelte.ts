import { usePlatform } from '#lib/api/client.js';
import { BUILT_IN_STYLES, isBuiltIn } from '#lib/assets/styles/builtin.js';
import { PersistedRecord } from './persisted.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * A named set of overrides on top of what the client ships
 */
export interface Style {
	id: string;
	name: string;
	author?: string;
	tokens: Record<string, string>;
	/** Arbitrary CSS, applied across the whole client. */
	css: string;
}

interface StyleLibrary {
	active: string | null;
	styles: Style[];
}

function defaults(): StyleLibrary {
	return { active: null, styles: [] };
}

/** The one `<style>` element the styling owns. */
const STYLE_ID = 'lorealm-style';

const TOKEN_NAME = /^--[A-Za-z0-9_-]+$/;

function newId(): string {
	return `s${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
}

class StyleStore {
	#record = new PersistedRecord<StyleLibrary>('styles', defaults());

	async load(): Promise<void> {
		await this.#record.load();
		await this.#adopt();
	}

	get all(): Style[] {
		return this.#record.current.styles;
	}

	get library(): Style[] {
		return [...BUILT_IN_STYLES, ...this.all];
	}

	get editable(): boolean {
		return this.active !== undefined && !isBuiltIn(this.active.id);
	}

	get activeId(): string | null {
		return this.#record.current.active;
	}

	get active(): Style | undefined {
		return this.library.find((s) => s.id === this.activeId);
	}

	get tokens(): Record<string, string> {
		return this.active?.tokens ?? {};
	}

	get css(): string {
		return this.active?.css ?? '';
	}

	select(id: string | null): Promise<void> {
		return this.#record.set('active', id);
	}

	async add(style: Style): Promise<void> {
		await this.#record.set('styles', [...this.all, style]);
		await this.select(style.id);
	}

	create(name: string = t.settings.appearance.newStyle): Promise<void> {
		return this.add({ id: newId(), name, tokens: {}, css: '' });
	}

	async remove(id: string): Promise<void> {
		await this.#record.set(
			'styles',
			this.all.filter((s) => s.id !== id)
		);
		if (this.activeId === id) await this.select(null);
	}

	setToken(name: string, value: string): Promise<void> {
		return this.#fork((s) => ({ ...s, tokens: { ...s.tokens, [name]: value } }));
	}

	setCss(css: string): Promise<void> {
		return this.#fork((s) => ({ ...s, css }));
	}

	removeToken(name: string): Promise<void> {
		return this.#fork((s) => ({
			...s,
			tokens: Object.fromEntries(Object.entries(s.tokens).filter(([key]) => key !== name))
		}));
	}

	rename(name: string): Promise<void> {
		return this.#update((s) => ({ ...s, name }));
	}

	setAuthor(author: string): Promise<void> {
		return this.#update((s) => ({ ...s, author: author.trim() === '' ? undefined : author }));
	}

	reset(): Promise<void> {
		return this.#update((s) => ({ ...s, tokens: {}, css: '' }));
	}

	#update(change: (style: Style) => Style): Promise<void> {
		const active = this.active;
		if (!active || isBuiltIn(active.id)) return Promise.resolve();

		return this.#record.set(
			'styles',
			this.all.map((s) => (s.id === active.id ? change(s) : s))
		);
	}

	#fork(change: (style: Style) => Style): Promise<void> {
		const active = this.active;
		if (active && !isBuiltIn(active.id)) return this.#update(change);

		const base: Style = active
			? {
					...active,
					id: newId(),
					name: t.settings.appearance.copyOf(active.name),
					tokens: { ...active.tokens }
				}
			: { id: newId(), name: t.settings.appearance.custom, tokens: {}, css: '' };

		return this.add(change(base));
	}

	async #adopt(): Promise<void> {
		const store = usePlatform().store;
		const old = await store.get<Partial<Style>>('theme');
		if (!old) return;

		const tokens = old.tokens ?? {};
		const css = old.css ?? '';
		if (Object.keys(tokens).length > 0 || css !== '') {
			await this.add({ id: newId(), name: t.settings.appearance.custom, tokens, css });
		}
		await store.delete('theme');
	}
}

export const styles = new StyleStore();

/**
 * Write the style into the document.
 */
export function applyStyle(tokens: Record<string, string>, css: string): void {
	const declarations = Object.entries(tokens)
		.filter(([name, value]) => TOKEN_NAME.test(name) && !/[;{}]/.test(value))
		.map(([name, value]) => `\t${name}: ${value};`)
		.join('\n');

	const element = document.getElementById(STYLE_ID) ?? document.createElement('style');
	element.id = STYLE_ID;
	element.textContent = `:root {\n${declarations}\n}\n\n${css}`;

	document.head.appendChild(element);
	tintBrowser();
}

function tintBrowser(): void {
	const ground = getComputedStyle(document.documentElement)
		.getPropertyValue('--backdrop-ground')
		.trim();
	if (!ground) return;
	let meta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');
	if (!meta) {
		meta = document.createElement('meta');
		meta.name = 'theme-color';
		document.head.appendChild(meta);
	}
	meta.content = ground;
}

export function styleFile(style: Style): string {
	const contents = {
		name: style.name,
		author: style.author,
		tokens: style.tokens,
		css: style.css
	};
	return `${JSON.stringify(contents, null, '\t')}\n`;
}

/**
 * Read a style file.
 */
export function parseStyle(text: string): Style {
	let data: unknown;
	try {
		data = JSON.parse(text);
	} catch {
		throw new Error(t.errors.failed.styleNotJson);
	}

	if (typeof data !== 'object' || data === null || Array.isArray(data)) {
		throw new Error(t.errors.failed.styleNotObject);
	}

	const raw = data as Record<string, unknown>;
	if (typeof raw.name !== 'string' || raw.name.trim() === '') {
		throw new Error(t.errors.failed.styleNoName);
	}
	if (raw.css !== undefined && typeof raw.css !== 'string') {
		throw new Error(t.errors.failed.styleCss);
	}

	const tokens: Record<string, string> = {};
	if (raw.tokens !== undefined) {
		if (typeof raw.tokens !== 'object' || raw.tokens === null || Array.isArray(raw.tokens)) {
			throw new Error(t.errors.failed.styleTokens);
		}
		for (const [name, value] of Object.entries(raw.tokens)) {
			if (typeof value !== 'string') throw new Error(t.errors.failed.styleTokenValue(name));
			tokens[name] = value;
		}
	}

	return {
		id: newId(),
		name: raw.name.trim(),
		author: typeof raw.author === 'string' && raw.author.trim() !== '' ? raw.author : undefined,
		tokens,
		css: typeof raw.css === 'string' ? raw.css : ''
	};
}

/** A filename for a style, safe on every platform we target. */
export function styleFilename(style: Style): string {
	const stem = style.name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
	return `${stem === '' ? 'style' : stem}.json`;
}

/**
 * Every token the stylesheets declare on `:root`, keyed by name.
 */
export function tokenCatalogue(): Record<string, string> {
	const found: Record<string, string> = {};

	for (const sheet of Array.from(document.styleSheets)) {
		if (sheet.ownerNode instanceof Element && sheet.ownerNode.id === STYLE_ID) continue;

		let rules: CSSRuleList;
		try {
			rules = sheet.cssRules;
		} catch {
			continue; // A cross-origin sheet. Nothing readable here.
		}

		for (const rule of Array.from(rules)) {
			if (!(rule instanceof CSSStyleRule) || rule.selectorText !== ':root') continue;

			for (let i = 0; i < rule.style.length; i++) {
				const name = rule.style.item(i);
				if (name.startsWith('--')) found[name] = rule.style.getPropertyValue(name).trim();
			}
		}
	}

	return Object.fromEntries(Object.entries(found).sort(([a], [b]) => a.localeCompare(b)));
}
