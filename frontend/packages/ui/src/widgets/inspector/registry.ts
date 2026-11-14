import type { Component } from 'svelte';
import AuthorNoteWidget from './widgets/AuthorNoteWidget.svelte';
import CharacterImageWidget from './widgets/CharacterImageWidget.svelte';
import ContextWidget from './widgets/ContextWidget.svelte';
import CreatorNotesWidget from './widgets/CreatorNotesWidget.svelte';
import RealmAgentWidget from './widgets/RealmAgentWidget.svelte';
import StateWidget from './widgets/StateWidget.svelte';
import TemplateWidget from './widgets/TemplateWidget.svelte';
import { t } from '#lib/stores/i18n.svelte.js';

export type BuiltinWidgetKey =
	| 'character-image'
	| 'template'
	| 'context'
	| 'state'
	| 'realm-agent'
	| 'author-note'
	| 'creator-notes';

/** A plugin's inspector slot. */
export type PluginWidgetKey = `plugin:${string}:${string}`;

export type InspectorWidgetKey = BuiltinWidgetKey | PluginWidgetKey;

export const inspectorRegistry: Record<BuiltinWidgetKey, { label: string; component: Component }> =
	{
		'character-image': {
			get label() {
				return t.inspector.widgets.characterImage;
			},
			component: CharacterImageWidget
		},
		template: {
			get label() {
				return t.inspector.widgets.template;
			},
			component: TemplateWidget
		},
		context: {
			get label() {
				return t.inspector.widgets.context;
			},
			component: ContextWidget
		},
		state: {
			get label() {
				return t.inspector.widgets.state;
			},
			component: StateWidget
		},
		'realm-agent': {
			get label() {
				return t.inspector.widgets.realmAgent;
			},
			component: RealmAgentWidget
		},
		'author-note': {
			get label() {
				return t.inspector.widgets.authorNote;
			},
			component: AuthorNoteWidget
		},
		'creator-notes': {
			get label() {
				return t.inspector.widgets.creatorNotes;
			},
			component: CreatorNotesWidget
		}
	};

/** Catalogue order, used by the cell editor. */
export const inspectorWidgetKeys = Object.keys(inspectorRegistry) as BuiltinWidgetKey[];

export function isInspectorWidgetKey(key: string): key is BuiltinWidgetKey {
	return key in inspectorRegistry;
}

export function pluginWidgetKey(handle: string, entry: string): PluginWidgetKey {
	return `plugin:${handle}:${entry}`;
}

export function isPluginWidgetKey(key: InspectorWidgetKey): key is PluginWidgetKey {
	return key.startsWith('plugin:');
}
