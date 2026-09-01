import { navigator } from '$lib/navigation/navigator.js';
import { router } from '$lib/navigation/router.svelte.js';
import { chatStore } from '$lib/stores/chat.svelte.js';
import { inspectorMatrix } from '$lib/stores/inspectorMatrix.svelte.js';
import { GLOBAL_LAYER, layers } from './layers.svelte.js';
import type { Command } from './types.js';

export const CHAT_COMPOSER_LAYER = 'chat.composer';

export interface ChatComposerLayer {
	focus(): void;
	send(): void;
}

function composer(): ChatComposerLayer | undefined {
	return layers.get(CHAT_COMPOSER_LAYER)?.data as ChatComposerLayer | undefined;
}

function branchTarget() {
	if (chatStore.isStreaming) return null;
	return chatStore.lastMessage;
}

function inspectorProfiles() {
	return inspectorMatrix.ordered(router.current.view);
}

function cycleInspectorProfile(delta: 1 | -1) {
	const profiles = inspectorProfiles();
	const at = profiles.findIndex((profile) => profile.id === inspectorMatrix.activeId);
	const next = (at + delta + profiles.length) % profiles.length;
	void inspectorMatrix.select(profiles[next].id);
}

/**
 * The shortcuts that ship with the app.
 *
 * Escape is deliberately unbound.
 */
export const builtInCommands: Command[] = [
	{
		id: 'nav.back',
		title: 'Back',
		description: 'Return to the previous destination.',
		group: 'Navigation',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+[', 'alt+ArrowLeft'],
		enabled: () => router.canGoBack,
		run: () => void navigator.back()
	},
	{
		id: 'nav.forward',
		title: 'Forward',
		description: 'Undo a Back.',
		group: 'Navigation',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+]', 'alt+ArrowRight'],
		enabled: () => router.canGoForward,
		run: () => void navigator.forward()
	},
	{
		id: 'view.write',
		title: 'Go to Write',
		group: 'Views',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+1'],
		run: () => navigator.start().write()
	},
	{
		id: 'view.characters',
		title: 'Go to Characters',
		group: 'Views',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+2'],
		run: () => navigator.start().characters()
	},
	{
		id: 'view.realms',
		title: 'Go to Realms',
		group: 'Views',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+3'],
		run: () => navigator.start().realms()
	},
	{
		id: 'view.connections',
		title: 'Go to Connections',
		group: 'Views',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+4'],
		run: () => navigator.start().connections()
	},
	{
		id: 'view.studio',
		title: 'Go to Studio',
		group: 'Views',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+5'],
		run: () => void navigator.start().studio()
	},
	{
		id: 'settings.open',
		title: 'Open settings',
		group: 'Settings',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+,'],
		run: () => void navigator.start().settings()
	},
	{
		id: 'keys.help',
		title: 'Keyboard shortcuts',
		description: 'Open the shortcut list.',
		group: 'Settings',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['F1'],
		run: () => navigator.start().settings().client().keybinds()
	},
	{
		id: 'chat.focus',
		title: 'Focus the composer',
		description: 'Put the cursor in the chat box.',
		group: 'Chat',
		scope: 'view',
		layer: 'write',
		keys: ['mod+l'],
		enabled: () => composer() !== undefined,
		run: () => composer()?.focus()
	},
	{
		id: 'chat.send',
		title: 'Send message',
		group: 'Chat',
		scope: 'module',
		layer: CHAT_COMPOSER_LAYER,
		keys: ['mod+Enter'],
		allowInEditable: true,
		run: () => composer()?.send()
	},
	{
		id: 'chat.branchPrev',
		title: 'Previous branch',
		description: 'Swipe the newest message back to its previous variant.',
		group: 'Chat',
		scope: 'view',
		layer: 'write',
		keys: ['ArrowLeft'],
		enabled: () => {
			const message = branchTarget();
			return message !== null && chatStore.siblingInfo(message.id).index > 0;
		},
		run: () => {
			const message = branchTarget();
			if (message) void chatStore.swipePrev(message.id);
		}
	},
	{
		id: 'chat.branchNext',
		title: 'Next branch',
		description:
			'Swipe the newest message forward, generating a fresh reply once past the last variant.',
		group: 'Chat',
		scope: 'view',
		layer: 'write',
		keys: ['ArrowRight'],
		enabled: () => {
			const message = branchTarget();
			if (!message) return false;
			const { index, count } = chatStore.siblingInfo(message.id);
			return index < count - 1 || message.role === 'assistant';
		},
		run: () => {
			const message = branchTarget();
			if (message) void chatStore.swipeNext(message.id);
		}
	},
	{
		id: 'inspector.nextProfile',
		title: 'Next inspector profile',
		group: 'Inspector',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: [']'],
		enabled: () => inspectorProfiles().length > 1,
		run: () => cycleInspectorProfile(1)
	},
	{
		id: 'inspector.prevProfile',
		title: 'Previous inspector profile',
		group: 'Inspector',
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['['],
		enabled: () => inspectorProfiles().length > 1,
		run: () => cycleInspectorProfile(-1)
	}
];
