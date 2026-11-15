import { navigator } from '#lib/navigation/navigator.js';
import { router } from '#lib/navigation/router.svelte.js';
import { characterStore } from '#lib/stores/character.svelte.js';
import { chatStore } from '#lib/stores/chat.svelte.js';
import { conversationManager } from '#lib/stores/conversationManager.svelte.js';
import type { SlashCommand } from './types.js';
import { t } from '#lib/stores/i18n.svelte.js';

const VIEWS: Record<string, () => void> = {
	write: () => navigator.start().write(),
	characters: () => navigator.start().characters(),
	realms: () => navigator.start().realms(),
	connections: () => navigator.start().connections(),
	studio: () => void navigator.start().studio(),
	settings: () => void navigator.start().settings()
};

/**
 * The commands that ship with the app.
 */
export const builtInSlashCommands: SlashCommand[] = [
	{
		name: 'send',
		get description() {
			return t.commands.send.description;
		},
		get restLabel() {
			return t.commands.send.rest;
		},
		raw: true,
		enabled: () => !chatStore.isStreaming,
		get unavailable() {
			return t.commands.send.unavailable;
		},
		run: async ({ rest }) => {
			if (rest) await chatStore.sendMessage(rest);
		}
	},
	{
		name: 'newchat',
		get description() {
			return t.commands.newchat.description;
		},
		enabled: () => characterStore.activeCharacter !== null,
		get unavailable() {
			return t.commands.noCharacter;
		},
		run: () => {
			const character = characterStore.activeCharacter;
			if (!character) return;
			chatStore.beginNewConversation({
				characterId: character.id,
				versionId: character.current_version_id,
				greetings: character.definition.greetings
			});
		}
	},
	{
		name: 'conversations',
		aliases: ['convos'],
		get description() {
			return t.commands.conversations.description;
		},
		enabled: () => chatStore.characterId !== null,
		get unavailable() {
			return t.commands.noCharacter;
		},
		run: () => {
			const characterId = chatStore.characterId;
			if (characterId) conversationManager.open(characterId);
		}
	},
	{
		name: 'go',
		get description() {
			return t.commands.go.description({ views: Object.keys(VIEWS).join(', ') });
		},
		get restLabel() {
			return t.commands.go.rest;
		},
		run: ({ rest }) => {
			const open = VIEWS[rest.trim().toLowerCase()];
			if (!open) throw new Error(t.commands.go.unknownView({ view: rest.trim() }));
			open();
		}
	},
	{
		name: 'back',
		get description() {
			return t.commands.back.description;
		},
		enabled: () => router.canGoBack,
		get unavailable() {
			return t.commands.back.unavailable;
		},
		run: () => void navigator.back()
	}
];
