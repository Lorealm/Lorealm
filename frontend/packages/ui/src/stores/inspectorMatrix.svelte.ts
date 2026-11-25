import type { ViewName } from '#lib/navigation/routes.js';
import type { InspectorWidgetKey } from '#lib/widgets/inspector/registry.js';
import { PersistedRecord } from './persisted.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * One column of the inspector matrix
 */
export interface InspectorProfile {
	id: string;
	name: string;
	/** A name from the icon catalogue (see `assets/icons/catalogue.ts`). */
	icon: string;
	cells: Partial<Record<ViewName, InspectorWidgetKey[]>>;
}

interface InspectorMatrix {
	profiles: InspectorProfile[];
	activeId: string;
}

function defaults(): InspectorMatrix {
	return {
		profiles: [
			{
				id: 'overview',
				name: t.settings.inspector.defaultProfile,
				icon: 'layout-panel-left',
				cells: {
					write: ['character-image', 'author-note', 'template'],
					characters: ['creator-notes'],
					realms: ['realm-agent'],
					studio: ['style-variables']
				}
			},
			{
				id: 'debug',
				name: t.settings.inspector.debugProfile,
				icon: 'bug',
				cells: {
					write: ['context', 'state'],
					realms: ['realm-agent'],
					studio: ['style-variables', 'context', 'state']
				}
			}
		],
		activeId: 'overview'
	};
}

class InspectorMatrixStore {
	#record = new PersistedRecord<InspectorMatrix>('inspectorMatrix', defaults());

	load(): Promise<void> {
		return this.#record.load();
	}

	get profiles(): InspectorProfile[] {
		return this.#record.current.profiles;
	}

	get activeId(): string {
		return this.#record.current.activeId;
	}

	get active(): InspectorProfile | undefined {
		return this.profiles.find((p) => p.id === this.activeId) ?? this.profiles[0];
	}

	widgetsFor(view: ViewName): InspectorWidgetKey[] {
		return this.active?.cells[view] ?? [];
	}

	isConfigured(profile: InspectorProfile, view: ViewName): boolean {
		return (profile.cells[view] ?? []).length > 0;
	}

	ordered(view: ViewName): InspectorProfile[] {
		const configured = this.profiles.filter((p) => this.isConfigured(p, view));
		const rest = this.profiles.filter((p) => !this.isConfigured(p, view));
		return [...configured, ...rest];
	}

	select(id: string): Promise<void> {
		return this.#record.set('activeId', id);
	}

	async addProfile(): Promise<string> {
		const profile: InspectorProfile = {
			// Not `crypto.randomUUID`: that needs a secure context, and a
			// self-hosted server may well be reached over plain HTTP.
			id: `p${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`,
			name: t.settings.inspector.profileName({ n: this.profiles.length + 1 }),
			icon: 'eye',
			cells: {}
		};
		await this.#record.set('profiles', [...this.profiles, profile]);
		return profile.id;
	}

	async removeProfile(id: string): Promise<void> {
		const remaining = this.profiles.filter((p) => p.id !== id);
		await this.#record.set('profiles', remaining);
		if (this.activeId === id && remaining.length > 0) await this.select(remaining[0].id);
	}

	#update(id: string, change: (profile: InspectorProfile) => InspectorProfile): Promise<void> {
		return this.#record.set(
			'profiles',
			this.profiles.map((p) => (p.id === id ? change(p) : p))
		);
	}

	rename(id: string, name: string): Promise<void> {
		return this.#update(id, (p) => ({ ...p, name }));
	}

	setIcon(id: string, icon: string): Promise<void> {
		return this.#update(id, (p) => ({ ...p, icon }));
	}

	setCell(id: string, view: ViewName, widgets: InspectorWidgetKey[]): Promise<void> {
		return this.#update(id, (p) => ({ ...p, cells: { ...p.cells, [view]: widgets } }));
	}

	#cell(id: string, view: ViewName): InspectorWidgetKey[] {
		return this.profiles.find((p) => p.id === id)?.cells[view] ?? [];
	}

	addWidget(id: string, view: ViewName, widget: InspectorWidgetKey): Promise<void> {
		const current = this.#cell(id, view);
		if (current.includes(widget)) return Promise.resolve();
		return this.setCell(id, view, [...current, widget]);
	}

	removeWidget(id: string, view: ViewName, index: number): Promise<void> {
		return this.setCell(
			id,
			view,
			this.#cell(id, view).filter((_, i) => i !== index)
		);
	}

	moveWidget(id: string, view: ViewName, index: number, delta: -1 | 1): Promise<void> {
		const next = [...this.#cell(id, view)];
		const to = index + delta;
		if (to < 0 || to >= next.length) return Promise.resolve();
		[next[index], next[to]] = [next[to], next[index]];
		return this.setCell(id, view, next);
	}
}

export const inspectorMatrix = new InspectorMatrixStore();
