// Mirror of the backend `TracedBlock` (the outermost layer), with `ResolvedBlock`
// flattened in. These shapes only arrive as plain JSON, so ids stay `string`.
//
// Content is now stored by *reference* where it can be deterministically rebuilt
// (messages, character parts), and only inlined verbatim where it can't. So
// `content` is often empty here — see `$lib/inspector/content.ts` for turning a
// block back into display text.

/** Normalised form of the backend `ContentRef` (externally-tagged) enum.
 *  Rust `Verbatim` -> `"Verbatim"`, `Message(id)` -> `{ Message: id }`, etc. */
export type ContentRef =
	| { kind: 'verbatim' }
	| { kind: 'message'; id: string }
	| { kind: 'character'; part: CharacterPart }
	| { kind: 'plugin'; source_id: string; data: unknown }
	/** Forward-compat: a ref shape this build doesn't know about. */
	| { kind: 'unknown' };

/** Normalised form of the backend `CharacterPart` enum. */
export type CharacterPart =
	| { part: 'definition' }
	| { part: 'personality' }
	| { part: 'scenario' }
	| { part: 'greeting'; order: number }
	| { part: 'example'; order: number }
	| { part: 'unknown' };

/** Matches the `#[serde(tag = "kind", rename_all = "snake_case")]` enum. */
export type OriginKind =
	| { kind: 'text' }
	| { kind: 'source'; source_id: string; node_id: string }
	| { kind: 'group_wrapper' };

export interface BlockOrigin {
	node_id: string;
	node_title: string;
	/** Enclosing group ids, outermost first. Empty = root-level. */
	path: string[];
	kind: OriginKind;
}

export interface TracedBlock {
	role: string;
	/** Inline text. Empty for recoverable refs; populated for `verbatim`. */
	content: string;
	content_ref: ContentRef;
	origin: BlockOrigin;
}

function isRecord(v: unknown): v is Record<string, unknown> {
	return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function parseCharacterPart(v: unknown): CharacterPart {
	if (v === 'Definition') return { part: 'definition' };
	if (v === 'Personality') return { part: 'personality' };
	if (v === 'Scenario') return { part: 'scenario' };
	if (isRecord(v)) {
		if (isRecord(v.Greeting) && typeof v.Greeting.order === 'number') {
			return { part: 'greeting', order: v.Greeting.order };
		}
		if (isRecord(v.Example) && typeof v.Example.order === 'number') {
			return { part: 'example', order: v.Example.order };
		}
	}
	return { part: 'unknown' };
}

function parseContentRef(v: unknown): ContentRef {
	if (v === 'Verbatim') return { kind: 'verbatim' };
	if (isRecord(v)) {
		if (typeof v.Message === 'string') return { kind: 'message', id: v.Message };
		if ('Character' in v) return { kind: 'character', part: parseCharacterPart(v.Character) };
		if (isRecord(v.Plugin)) {
			return {
				kind: 'plugin',
				source_id: typeof v.Plugin.source_id === 'string' ? v.Plugin.source_id : '',
				data: v.Plugin.data
			};
		}
	}
	return { kind: 'unknown' };
}

function parseOriginKind(v: unknown): OriginKind {
	if (!isRecord(v) || typeof v.kind !== 'string') return { kind: 'text' };
	switch (v.kind) {
		case 'source':
			return {
				kind: 'source',
				source_id: typeof v.source_id === 'string' ? v.source_id : '',
				node_id: typeof v.node_id === 'string' ? v.node_id : ''
			};
		case 'group_wrapper':
			return { kind: 'group_wrapper' };
		default:
			return { kind: 'text' };
	}
}

function parseBlock(v: unknown): TracedBlock | null {
	if (!isRecord(v) || typeof v.content !== 'string' || typeof v.role !== 'string') return null;
	const o = isRecord(v.origin) ? v.origin : {};
	return {
		role: v.role,
		content: v.content,
		content_ref: parseContentRef(v.content_ref),
		origin: {
			node_id: typeof o.node_id === 'string' ? o.node_id : '',
			node_title: typeof o.node_title === 'string' ? o.node_title : '',
			path: Array.isArray(o.path) ? o.path.filter((x): x is string => typeof x === 'string') : [],
			kind: parseOriginKind(o.kind)
		}
	};
}

/**
 * Pull the traced context blocks out of a message's `metadata`.
 *
 * - `null`  -> no `context` trace at all (user turns, or pre-trace history).
 * - `[]`    -> a trace is present but the pipeline emitted nothing.
 * - blocks  -> in tree order, exactly as the backend walk produced them.
 */
export function parseContextBlocks(metadata: unknown): TracedBlock[] | null {
	if (!isRecord(metadata) || !Array.isArray(metadata.context)) return null;
	return metadata.context.map(parseBlock).filter((b): b is TracedBlock => b !== null);
}

/** A run of consecutive blocks that were emitted by the same context node. */
export interface BlockGroup {
	/** `origin.node_id` shared by every member. */
	node_id: string;
	title: string;
	blocks: TracedBlock[];
}

/**
 * Collapse runs of adjacent blocks from the same source node into groups, so a
 * long stretch (chat history especially) reads as one foldable unit instead of
 * dozens of rows. Only *adjacent* same-node blocks merge — order is untouched,
 * and a node that appears in two separate runs yields two groups. Blocks with
 * no node id never merge into each other.
 */
export function groupAdjacentBlocks(blocks: TracedBlock[]): BlockGroup[] {
	const groups: BlockGroup[] = [];
	for (const block of blocks) {
		const key = block.origin.node_id;
		const last = groups[groups.length - 1];
		if (last && key !== '' && last.node_id === key) {
			last.blocks.push(block);
		} else {
			groups.push({ node_id: key, title: block.origin.node_title, blocks: [block] });
		}
	}
	return groups;
}
