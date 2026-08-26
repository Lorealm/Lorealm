import { base, keyName } from 'w3c-keyname';

/**
 * A normalized key combination, e.g. `mod+shift+k`.
 *
 * Canonical form is `[mod+][ctrl+][alt+][shift+][meta+]<key>`, modifiers always
 * in that order. `mod` is the platform's primary modifier: Cmd on macOS, Ctrl
 * everywhere else. {@link parseChord} folds the platform-specific spelling into
 * `mod`, so `ctrl+k` and `mod+k` are the same chord on Linux and conflict
 * detection sees them as one binding.
 */
export type Chord = string;

export const IS_MAC =
	typeof navigator !== 'undefined' && /Mac|iP(?:hone|ad|od)/.test(navigator.platform ?? '');

/** Pressing one of these on its own is not a chord. */
const MODIFIER_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta', 'AltGraph']);

const MODIFIER_ALIASES: Record<string, 'mod' | 'ctrl' | 'alt' | 'shift' | 'meta'> = {
	mod: 'mod',
	ctrl: 'ctrl',
	control: 'ctrl',
	alt: 'alt',
	opt: 'alt',
	option: 'alt',
	shift: 'shift',
	meta: 'meta',
	cmd: 'meta',
	command: 'meta',
	super: 'meta',
	win: 'meta'
};

/** Spellings we accept for keys whose canonical name is awkward to type. */
const KEY_ALIASES: Record<string, string> = {
	' ': 'Space',
	space: 'Space',
	spacebar: 'Space',
	esc: 'Escape',
	escape: 'Escape',
	enter: 'Enter',
	return: 'Enter',
	tab: 'Tab',
	backspace: 'Backspace',
	del: 'Delete',
	delete: 'Delete',
	ins: 'Insert',
	insert: 'Insert',
	home: 'Home',
	end: 'End',
	pageup: 'PageUp',
	pagedown: 'PageDown',
	up: 'ArrowUp',
	down: 'ArrowDown',
	left: 'ArrowLeft',
	right: 'ArrowRight',
	arrowup: 'ArrowUp',
	arrowdown: 'ArrowDown',
	arrowleft: 'ArrowLeft',
	arrowright: 'ArrowRight'
};

interface Modifiers {
	mod: boolean;
	ctrl: boolean;
	alt: boolean;
	shift: boolean;
	meta: boolean;
}

function noModifiers(): Modifiers {
	return { mod: false, ctrl: false, alt: false, shift: false, meta: false };
}

/** Canonical spelling for one key name. */
function normalizeKey(name: string): string {
	const alias = KEY_ALIASES[name.toLowerCase()];
	if (alias) return alias;
	if (name.length === 1) return name.toLowerCase();
	if (/^f\d{1,2}$/i.test(name)) return name.toUpperCase();
	return name;
}

function assemble(modifiers: Modifiers, key: string): Chord {
	let chord = '';
	if (modifiers.mod) chord += 'mod+';
	if (modifiers.ctrl) chord += 'ctrl+';
	if (modifiers.alt) chord += 'alt+';
	if (modifiers.shift) chord += 'shift+';
	if (modifiers.meta) chord += 'meta+';
	return chord + key;
}

function splitChord(input: string): string[] {
	const parts: string[] = [];
	let rest = input;
	for (;;) {
		const at = rest.indexOf('+');
		if (at === -1 || at === rest.length - 1) {
			parts.push(rest);
			return parts;
		}
		parts.push(rest.slice(0, at));
		rest = rest.slice(at + 1);
	}
}

/** Normalize a written chord, or `null` when it names no key. */
export function parseChord(input: string): Chord | null {
	const parts = splitChord(input.trim());
	const key = parts.pop()?.trim();
	if (!key) return null;

	const modifiers = noModifiers();
	for (const part of parts) {
		const modifier = MODIFIER_ALIASES[part.trim().toLowerCase()];
		if (!modifier) return null;
		modifiers[modifier] = true;
	}
	fold(modifiers);

	return assemble(modifiers, normalizeKey(key));
}

/** Collapse the platform's primary modifier into `mod`. */
function fold(modifiers: Modifiers) {
	if (IS_MAC && modifiers.meta) {
		modifiers.mod = true;
		modifiers.meta = false;
	} else if (!IS_MAC && modifiers.ctrl) {
		modifiers.mod = true;
		modifiers.ctrl = false;
	}
}

function chordFor(name: string, event: KeyboardEvent): Chord {
	const modifiers = noModifiers();
	modifiers.ctrl = event.ctrlKey;
	modifiers.meta = event.metaKey;
	modifiers.alt = event.altKey;
	modifiers.shift = event.shiftKey;
	fold(modifiers);
	return assemble(modifiers, normalizeKey(name));
}

/**
 * The chords one keystroke could mean, best guess first.
 */
export function chordsFromEvent(event: KeyboardEvent): Chord[] {
	if (MODIFIER_KEYS.has(event.key)) return [];

	const name = keyName(event);
	if (!name || name === 'Unidentified') return [];

	const chords = [chordFor(name, event)];

	const baseName = (base as Record<number, string | undefined>)[event.keyCode];
	if (baseName && baseName !== name && (event.altKey || event.ctrlKey || event.metaKey)) {
		const fallback = chordFor(baseName, event);
		if (fallback !== chords[0]) chords.push(fallback);
	}

	return chords;
}

const MODIFIER_SYMBOLS: Record<string, string> = {
	mod: IS_MAC ? '⌘' : 'Ctrl',
	ctrl: IS_MAC ? '⌃' : 'Ctrl',
	alt: IS_MAC ? '⌥' : 'Alt',
	shift: IS_MAC ? '⇧' : 'Shift',
	meta: IS_MAC ? '⌘' : 'Win'
};

const KEY_SYMBOLS: Record<string, string> = {
	ArrowUp: '↑',
	ArrowDown: '↓',
	ArrowLeft: '←',
	ArrowRight: '→',
	Enter: IS_MAC ? '↩' : 'Enter',
	Backspace: IS_MAC ? '⌫' : 'Backspace',
	Delete: IS_MAC ? '⌦' : 'Delete',
	Escape: IS_MAC ? '⎋' : 'Esc',
	Tab: IS_MAC ? '⇥' : 'Tab',
	Space: 'Space'
};

/** How a chord should read to the user: `⌘⇧K` on macOS, `Ctrl+Shift+K` elsewhere. */
export function formatChord(chord: Chord): string {
	const parts = splitChord(chord);
	const key = parts.pop() ?? '';
	const pieces = parts.map((part) => MODIFIER_SYMBOLS[part] ?? part);
	pieces.push(KEY_SYMBOLS[key] ?? (key.length === 1 ? key.toUpperCase() : key));
	return pieces.join(IS_MAC ? '' : '+');
}
