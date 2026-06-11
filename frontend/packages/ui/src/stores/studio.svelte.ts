export enum StudioSection {
	Realm,
	Lorebooks,
	Templates,
	Characters,
	Commands,
	Personas
}

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
}

export const studioStore = new StudioStore();
