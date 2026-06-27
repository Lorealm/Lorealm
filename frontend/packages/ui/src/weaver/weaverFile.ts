import { parse, parseDocument, type Document } from 'yaml';
import type { EntryMeta, LorebookConfig } from '../types/lorebook.js';

export class WeaverFileError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'WeaverFileError';
	}
}

export interface ParsedEntry {
	doc: Document;
	meta: EntryMeta;
	body: string;
}

const KNOWN_KEYS = [
	'id',
	'name',
	'keywords',
	'regex',
	'condition',
	'scan_depth',
	'constant',
	'priority',
	'slot',
	'fallback',
	'insertion_order',
	'enabled',
	'sticky_turns',
	'cooldown',
	'group',
	'tags'
] as const;

// ── split / join ────────────────────────────────────────────────────────────

/** Split a `.weaver` source into its frontmatter YAML and body. Mirrors the
 *  server's delimiter requirement: a leading `---` line and a closing `---`
 *  line are mandatory. CRLF is normalized to LF. */
export function splitFrontmatter(source: string): { frontmatter: string; body: string } {
	const lines = source.split(/\r?\n/);
	if (lines[0]?.trim() !== '---') {
		throw new WeaverFileError('missing frontmatter delimiters (---)');
	}
	let close = -1;
	for (let i = 1; i < lines.length; i++) {
		if (lines[i].trim() === '---') {
			close = i;
			break;
		}
	}
	if (close === -1) {
		throw new WeaverFileError('missing closing frontmatter delimiter (---)');
	}
	return {
		frontmatter: lines.slice(1, close).join('\n'),
		body: lines.slice(close + 1).join('\n')
	};
}

// ── parse ─────────────────────────────────────────────────────────────────

export function parseEntry(source: string): ParsedEntry {
	const { frontmatter, body } = splitFrontmatter(source);
	const doc = parseDocument(frontmatter);
	if (doc.errors.length > 0) {
		throw new WeaverFileError(doc.errors[0].message);
	}
	const raw = (doc.toJS() ?? {}) as Record<string, unknown>;
	if (typeof raw.id !== 'string') {
		throw new WeaverFileError("frontmatter missing required string field 'id'");
	}
	// The view mirrors the document exactly: known keys present in the source
	// carry their value, absent ones stay undefined. We deliberately do not
	// inject defaults — that is the form's job, at display time.
	const meta: EntryMeta = { id: raw.id };
	for (const key of KNOWN_KEYS) {
		if (key === 'id') continue;
		if (key in raw) (meta as unknown as Record<string, unknown>)[key] = raw[key];
	}
	return { doc, meta, body };
}

/** Parse `lorebook.yaml` into the config view. Unlike entries this is NOT
 *  round-tripped: the editor sends the whole structured object back as
 *  `UpdateLorebookRequest.config` and the server re-serializes, so extension
 *  keys survive by virtue of being parsed into the object. */
export function parseConfig(configYaml: string): LorebookConfig {
	const parsed = parse(configYaml);
	if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
		throw new WeaverFileError('lorebook config is not a YAML mapping');
	}
	return parsed as LorebookConfig;
}

// ── serialize ─────────────────────────────────────────────────────────────

/** Re-emit a `.weaver` source from an edited view. Only frontmatter keys whose
 *  value differs from the original document are written; unknown keys, comments,
 *  formatting, and key order are preserved. The body is written verbatim. */
export function serializeEntry(parsed: ParsedEntry): string {
	const { doc, meta, body } = parsed;
	const out = doc.clone() as Document;
	const orig = (doc.toJS() ?? {}) as Record<string, unknown>;

	for (const key of KNOWN_KEYS) {
		const next = (meta as unknown as Record<string, unknown>)[key];
		const prev = orig[key];
		if (deepEqual(next, prev)) continue; // untouched — leave the node alone

		if (isAbsent(next)) {
			out.delete(key);
		} else {
			out.set(key, next); // updates in place: key order and comments survive
		}
	}

	const fm = out.toString().replace(/\n$/, ''); // doc.toString() adds a newline
	return `---\n${fm}\n---\n${body}`;
}

// ── helpers ─────────────────────────────────────────────────────────────────

/** A value the author has effectively cleared → delete the key. Empty arrays
 *  are handled by the collection branch (written as `[]`), not treated as
 *  absent, so an explicit "no keywords" is distinguishable in the source. */
function isAbsent(v: unknown): boolean {
	return v === undefined || v === null || v === '';
}

/** Structural equality for the plain values frontmatter holds (scalars, string
 *  arrays, and the `{ at_depth: [n] }` slot object). Both-undefined is equal. */
function deepEqual(a: unknown, b: unknown): boolean {
	if (a === b) return true;
	if (a === undefined || b === undefined || a === null || b === null) return a === b;
	if (typeof a !== typeof b) return false;
	if (typeof a !== 'object') return false;
	return JSON.stringify(a) === JSON.stringify(b);
}
