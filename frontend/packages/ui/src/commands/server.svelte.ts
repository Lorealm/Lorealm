import { useApi } from '#lib/api/client.js';
import { CHAT_COMPOSER_LAYER, type ChatComposerLayer } from '#lib/keys/commands.js';
import { layers } from '#lib/keys/layers.svelte.js';
import { chatStore } from '#lib/stores/chat.svelte.js';
import { showToast } from '#lib/stores/errorToast.svelte.js';
import { messageStateStore } from '#lib/stores/messageState.svelte.js';
import { pluginCalls } from '#lib/stores/pluginCalls.svelte.js';
import type { Snowflake } from '#lib/types/core.js';
import type { CommandEffect, CommandRun, UserTool } from '#lib/types/tools.js';
import { slashCommands } from './registry.svelte.js';
import type { SlashCommand } from './types.js';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * Names a server command may not take, mirroring the server's own rule.
 */
const RESERVED_NAMES = new Set(['send']);

/**
 * Server-backed weaver commands, mirrored into the composer's registry.
 */
class ServerCommands {
	#unregister: (() => void) | null = null;

	/** The command currently round-tripping, if any. */
	running = $state<{ name: string; conversationId: Snowflake } | null>(null);

	/** Stop a running command. */
	async cancel(): Promise<void> {
		const inFlight = this.running;
		if (!inFlight) return;
		try {
			await this.#api.chat.cancel(inFlight.conversationId);
		} catch (e) {
			console.warn('Failed to cancel command', e);
		}
	}

	get #api() {
		return useApi();
	}

	/** Fetch the user's commands and mirror them into the registry. */
	async load(): Promise<void> {
		let tools: UserTool[];
		try {
			tools = await this.#api.tools.commands();
		} catch (e) {
			console.warn('Failed to load server commands', e);
			return;
		}

		// Restores any built-in a previous load was shadowing, then re-shadows.
		this.#unregister?.();
		this.#unregister = slashCommands.register(
			tools.filter((tool) => !RESERVED_NAMES.has(tool.name)).map(toSlashCommand)
		);
	}

	/** Drop them, e.g. on sign-out. */
	clear(): void {
		this.#unregister?.();
		this.#unregister = null;
	}
}

function toSlashCommand(tool: UserTool): SlashCommand {
	return {
		name: tool.name,
		description: tool.description || (tool.display_name ?? tool.name),
		params: tool.params.map((p) => ({
			name: p.name,
			description: p.description ?? '',
			required: p.required,
			resource: p.resource
		})),
		get restLabel() {
			return t.commands.server.rest;
		},
		enabled: () =>
			chatStore.conversationId !== null &&
			serverCommands.running === null &&
			pluginCalls.waitingOn(chatStore.conversationId) === null &&
			!chatStore.isStreaming,
		get unavailable() {
			return t.commands.server.unavailable;
		},
		run: async ({ named, rest }) => {
			const conversationId = chatStore.conversationId;
			if (!conversationId) return;
			serverCommands.running = { name: tool.name, conversationId };
			try {
				const run = await useApi().tools.run(tool.id, {
					conversation_id: conversationId,
					named,
					rest
				});
				applyRun(run);
			} finally {
				serverCommands.running = null;
			}
		}
	};
}

/** Carry out whatever the body asked for. */
export function applyEffect(effect: CommandEffect): void {
	switch (effect.type) {
		case 'toast':
			showToast({ message: effect.content, variant: effect.variant });
			break;
		case 'compose': {
			const composer = layers.get(CHAT_COMPOSER_LAYER)?.data as ChatComposerLayer | undefined;
			composer?.draft(effect.content, effect.mode);
			break;
		}
	}
}

function applyRun(run: CommandRun): void {
	for (const effect of run.effects) applyEffect(effect);

	if (run.message) {
		chatStore.ingestServerMessage(run.message);
		messageStateStore.inspect(run.message.id, run.message.conversation_id);
	}
}

export const serverCommands = new ServerCommands();
