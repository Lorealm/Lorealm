// Ordering for the entry list. Two rules, in order:
//   1. Enabled entries always sort before disabled ones, whatever the key.
//   2. Within each group, sort by the chosen key.
// A stable tiebreak on `id` keeps the order deterministic. Kept pure (operates
// on `{ meta }`, returns a new array) so it is unit-testable without the store.

import type { EntryMeta } from '../types/lorebook.ts';

export type SortKey = 'name' | 'priority' | 'insertion_order' | 'id';

export const SORT_KEYS: { key: SortKey; label: string }[] = [
	{ key: 'name', label: 'Name' },
	{ key: 'priority', label: 'Priority' },
	{ key: 'insertion_order', label: 'Insertion order' },
	{ key: 'id', label: 'ID' }
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
		if (ea !== eb) return ea ? -1 : 1; // enabled first

		const byKey = compareKey(a.meta, b.meta, key, dp);
		if (byKey !== 0) return byKey;
		return a.meta.id.localeCompare(b.meta.id); // deterministic tiebreak
	});
}

function compareKey(a: EntryMeta, b: EntryMeta, key: SortKey, dp: number): number {
	switch (key) {
		case 'priority':
			return (b.priority ?? dp) - (a.priority ?? dp); // higher priority first
		case 'insertion_order':
			return (a.insertion_order ?? 0) - (b.insertion_order ?? 0); // lower first
		case 'id':
			return a.id.localeCompare(b.id);
		case 'name':
		default:
			// fall back to id when unnamed; case-insensitive
			return (a.name ?? a.id).toLowerCase().localeCompare((b.name ?? b.id).toLowerCase());
	}
}
