import { navigator } from '#lib/navigation/navigator.js';
import { router } from '#lib/navigation/router.svelte.js';
import { chatStore } from '#lib/stores/chat.svelte.js';
import { conversationManager } from '#lib/stores/conversationManager.svelte.js';
import { inspectorMatrix } from '#lib/stores/inspectorMatrix.svelte.js';
import { GLOBAL_LAYER, layers } from './layers.svelte.js';
import type { Command } from './types.js';
import { t } from '#lib/stores/i18n.svelte.js';

export const CHAT_COMPOSER_LAYER = 'chat.composer';
export const CONVERSATION_MANAGER_LAYER = 'chat.conversationManager';

export interface ChatComposerLayer {
	focus(): void;
	send(): void;
	/** Put text in the box, replacing the draft or adding to it. */
	draft(content: string, mode: 'replace' | 'append'): void;
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
		get title() {
			return t.keys.commands.navBack.title;
		},
		get description() {
			return t.keys.commands.navBack.description;
		},
		get group() {
			return t.keys.groups.navigation;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+[', 'alt+ArrowLeft'],
		enabled: () => router.canGoBack,
		run: () => void navigator.back()
	},
	{
		id: 'nav.forward',
		get title() {
			return t.keys.commands.navForward.title;
		},
		get description() {
			return t.keys.commands.navForward.description;
		},
		get group() {
			return t.keys.groups.navigation;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+]', 'alt+ArrowRight'],
		enabled: () => router.canGoForward,
		run: () => void navigator.forward()
	},
	{
		id: 'view.write',
		get title() {
			return t.keys.commands.viewWrite.title;
		},
		get group() {
			return t.keys.groups.views;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+1'],
		run: () => navigator.start().write()
	},
	{
		id: 'view.characters',
		get title() {
			return t.keys.commands.viewCharacters.title;
		},
		get group() {
			return t.keys.groups.views;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+2'],
		run: () => navigator.start().characters()
	},
	{
		id: 'view.realms',
		get title() {
			return t.keys.commands.viewRealms.title;
		},
		get group() {
			return t.keys.groups.views;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+3'],
		run: () => navigator.start().realms()
	},
	{
		id: 'view.connections',
		get title() {
			return t.keys.commands.viewConnections.title;
		},
		get group() {
			return t.keys.groups.views;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+4'],
		run: () => navigator.start().connections()
	},
	{
		id: 'view.studio',
		get title() {
			return t.keys.commands.viewStudio.title;
		},
		get group() {
			return t.keys.groups.views;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['alt+5'],
		run: () => void navigator.start().studio()
	},
	{
		id: 'settings.open',
		get title() {
			return t.keys.commands.settingsOpen.title;
		},
		get group() {
			return t.keys.groups.settings;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+,'],
		run: () => void navigator.start().settings()
	},
	{
		id: 'keys.help',
		get title() {
			return t.keys.commands.keysHelp.title;
		},
		get description() {
			return t.keys.commands.keysHelp.description;
		},
		get group() {
			return t.keys.groups.settings;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['F1'],
		run: () => navigator.start().settings().user().keybinds()
	},
	{
		id: 'chat.focus',
		get title() {
			return t.keys.commands.chatFocus.title;
		},
		get description() {
			return t.keys.commands.chatFocus.description;
		},
		get group() {
			return t.keys.groups.chat;
		},
		scope: 'view',
		layer: 'write',
		keys: ['mod+l'],
		enabled: () => composer() !== undefined,
		run: () => composer()?.focus()
	},
	{
		id: 'chat.send',
		get title() {
			return t.keys.commands.chatSend.title;
		},
		get group() {
			return t.keys.groups.chat;
		},
		scope: 'module',
		layer: CHAT_COMPOSER_LAYER,
		keys: ['mod+Enter'],
		allowInEditable: true,
		run: () => composer()?.send()
	},
	{
		id: 'chat.conversations',
		get title() {
			return t.keys.commands.chatConversations.title;
		},
		get description() {
			return t.keys.commands.chatConversations.description;
		},
		get group() {
			return t.keys.groups.chat;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['mod+k'],
		allowInEditable: true,
		enabled: () => chatStore.characterId !== null,
		run: () => {
			const characterId = chatStore.characterId;
			if (characterId) conversationManager.open(characterId);
		}
	},
	{
		id: 'chat.conversations.close',
		get title() {
			return t.keys.commands.chatConversationsClose.title;
		},
		get group() {
			return t.keys.groups.chat;
		},
		scope: 'module',
		layer: CONVERSATION_MANAGER_LAYER,
		keys: ['mod+k'],
		allowInEditable: true,
		run: () => conversationManager.close()
	},
	{
		id: 'chat.branchPrev',
		get title() {
			return t.keys.commands.chatBranchPrev.title;
		},
		get description() {
			return t.keys.commands.chatBranchPrev.description;
		},
		get group() {
			return t.keys.groups.chat;
		},
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
		get title() {
			return t.keys.commands.chatBranchNext.title;
		},
		get description() {
			return t.keys.commands.chatBranchNext.description;
		},
		get group() {
			return t.keys.groups.chat;
		},
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
		get title() {
			return t.keys.commands.inspectorNextProfile.title;
		},
		get group() {
			return t.keys.groups.inspector;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: [']'],
		enabled: () => inspectorProfiles().length > 1,
		run: () => cycleInspectorProfile(1)
	},
	{
		id: 'inspector.prevProfile',
		get title() {
			return t.keys.commands.inspectorPrevProfile.title;
		},
		get group() {
			return t.keys.groups.inspector;
		},
		scope: 'global',
		layer: GLOBAL_LAYER,
		keys: ['['],
		enabled: () => inspectorProfiles().length > 1,
		run: () => cycleInspectorProfile(-1)
	}
];
