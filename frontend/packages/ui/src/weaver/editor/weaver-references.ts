import { Facet } from '@codemirror/state';
import { activeConstruct } from './weaver-context.js';

export interface EntryRef {
	id: string;
	name?: string;
	detail?: string;
	navigable?: boolean;
}

export interface ReferenceSource {
	entries: () => EntryRef[];
	/** Open `ref`. Omit to leave references non-navigable. */
	navigate?: (ref: EntryRef) => void;
}

export const referenceSource = Facet.define<ReferenceSource, ReferenceSource | null>({
	combine: (values) => values[0] ?? null
});

export type IdSite = 'doc' | 'trigger';

const IDENTIFIER = /^[A-Za-z_]\w*$/;

export function writableAt(site: IdSite, id: string): boolean {
	return site === 'trigger' || IDENTIFIER.test(id);
}

const OPEN_TRIGGER_ID = /<trigger\b[^>]*\bid\s*=\s*"[^"]*$/;

export function idSiteAt(before: string): IdSite | null {
	if (activeConstruct(before) === 'doc') return 'doc';
	if (OPEN_TRIGGER_ID.test(before)) return 'trigger';
	return null;
}

export interface IdRef {
	site: IdSite;
	id: string;
	from: number;
	to: number;
}

const DOC_REF = /\[\[\s*([A-Za-z_]\w*)\s*\]\]/dg;
const TRIGGER_REF = /<trigger\b[^>]*?\bid\s*=\s*"([^"\\]*)"/dg;

export function scanIdRefs(text: string): IdRef[] {
	const refs: IdRef[] = [];
	for (const [re, site] of [
		[DOC_REF, 'doc'],
		[TRIGGER_REF, 'trigger']
	] as const) {
		re.lastIndex = 0;
		let m: RegExpExecArray | null;
		while ((m = re.exec(text))) {
			const span = m.indices?.[1];
			if (span) refs.push({ site, id: m[1], from: span[0], to: span[1] });
		}
	}
	return refs.sort((a, b) => a.from - b.from);
}
