import { navigator } from '$lib/navigation/navigator.js';
import { router } from '$lib/navigation/router.svelte.js';
import { characterStore } from '$lib/stores/character.svelte.js';
import { chatStore } from '$lib/stores/chat.svelte.js';
import { conversationManager } from '$lib/stores/conversationManager.svelte.js';
import type { SlashCommand } from './types.js';

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
		description: 'Send a message and generate a reply.',
		restLabel: 'message',
		raw: true,
		enabled: () => !chatStore.isStreaming,
		unavailable: 'A reply is still streaming.',
		run: async ({ rest }) => {
			if (rest) await chatStore.sendMessage(rest);
		}
	},
	{
		name: 'newchat',
		description: 'Start a fresh conversation with the active character.',
		enabled: () => characterStore.activeCharacter !== null,
		unavailable: 'No character is active.',
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
		description: 'List the chats with the active character.',
		enabled: () => chatStore.characterId !== null,
		unavailable: 'No character is active.',
		run: () => {
			const characterId = chatStore.characterId;
			if (characterId) conversationManager.open(characterId);
		}
	},
	{
		name: 'go',
		description: `Open a view: ${Object.keys(VIEWS).join(', ')}.`,
		restLabel: 'view',
		run: ({ rest }) => {
			const open = VIEWS[rest.trim().toLowerCase()];
			if (!open) throw new Error(`Unknown view '${rest.trim()}'.`);
			open();
		}
	},
	{
		name: 'back',
		description: 'Return to the previous destination.',
		enabled: () => router.canGoBack,
		unavailable: 'There is nothing to go back to.',
		run: () => void navigator.back()
	}
];
