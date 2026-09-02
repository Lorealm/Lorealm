import { useApi } from '$lib/api/client.js';
import { CHAT_COMPOSER_LAYER, type ChatComposerLayer } from '$lib/keys/commands.js';
import { layers } from '$lib/keys/layers.svelte.js';
import { chatStore } from '$lib/stores/chat.svelte.js';
import { showToast } from '$lib/stores/errorToast.svelte.js';
import { messageStateStore } from '$lib/stores/messageState.svelte.js';
import type { Snowflake } from '$lib/types/core.js';
import type { CommandRun, UserTool } from '$lib/types/tools.js';
import { slashCommands } from './registry.svelte.js';
import type { SlashCommand } from './types.js';

/**
 * Server-backed weaver commands, mirrored into the composer's registry.
 *
 * The composer does not distinguish these from client commands — a `/name`
 * either resolves locally or round-trips, and the dispatcher cannot tell.
 * Client names win a collision, because `unregister` here never touches an
 * entry it did not add.
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
			// A failed load leaves only client commands available, which is a
			// degraded composer rather than a broken one.
			console.warn('Failed to load server commands', e);
			return;
		}

		this.#unregister?.();
		const claimed = new Set(
			slashCommands.all.filter((c) => c.source === 'core').map((c) => c.name)
		);
		this.#unregister = slashCommands.register(
			tools.filter((tool) => !claimed.has(tool.name)).map(toSlashCommand)
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
			required: p.required
		})),
		restLabel: 'text',
		// One at a time per conversation: cancellation tokens are keyed by
		// conversation id, so a command overlapping a streaming reply would
		// displace the stream's token and leave it unstoppable.
		enabled: () =>
			chatStore.conversationId !== null &&
			serverCommands.running === null &&
			!chatStore.isStreaming,
		unavailable: 'Wait for the current reply or command to finish.',
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
function applyRun(run: CommandRun): void {
	for (const effect of run.effects) {
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

	if (run.message) {
		chatStore.ingestServerMessage(run.message);
		messageStateStore.inspect(run.message.id, run.message.conversation_id);
	}
}

export const serverCommands = new ServerCommands();
