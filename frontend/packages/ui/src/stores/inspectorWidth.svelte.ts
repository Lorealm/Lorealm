import { PersistedRecord } from './persisted.svelte.js';

/** Narrow enough to stay a sidebar, wide enough for a table of grants. */
export const MIN_WIDTH = 230;
export const MAX_WIDTH = 720;

interface InspectorLayout {
	width: number;
}

const record = new PersistedRecord<InspectorLayout>('inspectorLayout', { width: 320 });

export const inspectorWidth = {
	get value(): number {
		return record.current.width;
	},
	load: () => record.load(),
	set(next: number) {
		void record.set('width', Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, next))));
	}
};
