import type { UserPersona } from './persona.js';

export type ContentRef =
	| { kind: 'verbatim' }
	| { kind: 'message'; id: string }
	| { kind: 'persona'; persona: UserPersona }
	| { kind: 'character'; part: CharacterPart }
	| { kind: 'plugin'; source_id: string; data: unknown }
	/** Future-proofing so deserialization doesn't break */
	| { kind: 'unknown' };

export type CharacterPart =
	| { part: 'definition' }
	| { part: 'personality' }
	| { part: 'scenario' }
	| { part: 'greeting'; order: number }
	| { part: 'example'; order: number }
	| { part: 'unknown' };

export type OriginKind =
	| { kind: 'text' }
	| { kind: 'source'; source_id: string; node_id: string }
	| { kind: 'group_wrapper' };

export interface BlockOrigin {
	node_id: string;
	node_title: string;
	path: string[];
	kind: OriginKind;
}

export interface TracedBlock {
	role: string;
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
		if (isRecord(v.Persona))
			return { kind: 'persona', persona: v.Persona as unknown as UserPersona };
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

export function parseContextBlocks(metadata: unknown): TracedBlock[] | null {
	if (!isRecord(metadata) || !Array.isArray(metadata.context)) return null;
	return metadata.context.map(parseBlock).filter((b): b is TracedBlock => b !== null);
}

/** A run of consecutive blocks that were emitted by the same context node. */
export interface BlockGroup {
	node_id: string;
	title: string;
	blocks: TracedBlock[];
}
/** Groups adjacent blocks of the same type */
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
