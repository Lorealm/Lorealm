import type { EntryMeta } from '../types/lorebook.ts';

export type SortKey = 'name' | 'priority' | 'insertion_order' | 'id';

export const SORT_KEYS: { key: SortKey }[] = [
	{ key: 'name' },
	{ key: 'priority' },
	{ key: 'insertion_order' },
	{ key: 'id' }
];

interface Sortable {
	meta: EntryMeta;
}

export function sortEntries<T extends Sortable>(
	entries: readonly T[],
	key: SortKey,
	defaults?: { priority?: number }
): T[] {
	const dp = defaults?.priority ?? 0;
	return [...entries].sort((a, b) => {
		const ea = a.meta.enabled ?? true;
		const eb = b.meta.enabled ?? true;
		if (ea !== eb) return ea ? -1 : 1;

		const byKey = compareKey(a.meta, b.meta, key, dp);
		if (byKey !== 0) return byKey;
		return a.meta.id.localeCompare(b.meta.id);
	});
}

function compareKey(a: EntryMeta, b: EntryMeta, key: SortKey, dp: number): number {
	switch (key) {
		case 'priority':
			return (b.priority ?? dp) - (a.priority ?? dp);
		case 'insertion_order':
			return (a.insertion_order ?? 0) - (b.insertion_order ?? 0);
		case 'id':
			return a.id.localeCompare(b.id);
		case 'name':
		default:
			// fall back to id when unnamed; case-insensitive
			return (a.name ?? a.id).toLowerCase().localeCompare((b.name ?? b.id).toLowerCase());
	}
}
