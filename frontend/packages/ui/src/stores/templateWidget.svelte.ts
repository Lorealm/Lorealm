import { PersistedRecord } from './persisted.svelte.js';

export type TemplateWidgetView = 'grid' | 'tree';

interface TemplateWidgetLayout {
	view: TemplateWidgetView;
}

const record = new PersistedRecord<TemplateWidgetLayout>('templateWidget', { view: 'grid' });

export const templateWidgetView = {
	get value(): TemplateWidgetView {
		return record.current.view;
	},
	load: () => record.load(),
	set(next: TemplateWidgetView) {
		void record.set('view', next);
	}
};
