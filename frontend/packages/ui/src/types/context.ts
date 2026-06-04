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
	content: string;
	origin: BlockOrigin;
}

function isRecord(v: unknown): v is Record<string, unknown> {
	return typeof v === 'object' && v !== null && !Array.isArray(v);
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
 * - `null`  → the message carries no `context` trace at all (user turns, or
 *             history saved before tracing existed). Callers use this to decide
 *             whether to show the inspector affordance.
 * - `[]`    → a trace is present but the pipeline emitted nothing.
 * - blocks  → in tree order, exactly as the backend walk produced them.
 */
export function parseContextBlocks(metadata: unknown): TracedBlock[] | null {
	if (!isRecord(metadata) || !Array.isArray(metadata.context)) return null;
	return metadata.context.map(parseBlock).filter((b): b is TracedBlock => b !== null);
}
