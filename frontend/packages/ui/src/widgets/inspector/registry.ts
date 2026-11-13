import type { Component } from 'svelte';
import AuthorNoteWidget from './widgets/AuthorNoteWidget.svelte';
import CharacterImageWidget from './widgets/CharacterImageWidget.svelte';
import ContextWidget from './widgets/ContextWidget.svelte';
import RealmAgentWidget from './widgets/RealmAgentWidget.svelte';
import StateWidget from './widgets/StateWidget.svelte';
import TemplateWidget from './widgets/TemplateWidget.svelte';

export type BuiltinWidgetKey =
	'character-image' | 'template' | 'context' | 'state' | 'realm-agent' | 'author-note';

/** A plugin's inspector slot. */
export type PluginWidgetKey = `plugin:${string}:${string}`;

export type InspectorWidgetKey = BuiltinWidgetKey | PluginWidgetKey;

export const inspectorRegistry: Record<BuiltinWidgetKey, { label: string; component: Component }> =
	{
		'character-image': { label: 'Character Image', component: CharacterImageWidget },
		template: { label: 'Template', component: TemplateWidget },
		context: { label: 'Context', component: ContextWidget },
		state: { label: 'State', component: StateWidget },
		'realm-agent': { label: 'Realm Agent', component: RealmAgentWidget },
		'author-note': { label: "Author's Note", component: AuthorNoteWidget }
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
