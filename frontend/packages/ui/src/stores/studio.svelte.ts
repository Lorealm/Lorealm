import { navigationStore, SidebarView } from './navigation.svelte';
import { toolStore } from './tools.svelte';
import type { Snowflake } from '$lib/types/core.js';

export enum StudioSection {
	Realm,
	Lorebooks,
	Templates,
	Characters,
	Tools,
	Personas
}

export type StudioResourceKind = 'character' | 'lorebook' | 'template' | 'tool' | 'persona';

const SECTION_FOR: Record<StudioResourceKind, StudioSection> = {
	character: StudioSection.Characters,
	lorebook: StudioSection.Lorebooks,
	template: StudioSection.Templates,
	tool: StudioSection.Tools,
	persona: StudioSection.Personas
};

class StudioStore {
	section = $state(StudioSection.Lorebooks);
	editingId = $state<string | null>(null);

	open(id: string) {
		this.editingId = id;
	}

	close() {
		this.editingId = null;
	}

	setSection(section: StudioSection) {
		this.section = section;
		this.editingId = null;
	}

	editResource(kind: StudioResourceKind, id: Snowflake) {
		this.setSection(SECTION_FOR[kind]);
		// The tools section drives its own selection instead of editingId.
		// TODO: fix this
		if (kind === 'tool') void toolStore.startEditing(id);
		else this.open(id);
		navigationStore.setSidebarView(SidebarView.Studio);
	}
}

export const studioStore = new StudioStore();
