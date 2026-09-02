import { SvelteMap } from 'svelte/reactivity';
import type { CommandSource } from '$lib/keys/types.js';
import type { RegisteredSlashCommand, SlashCommand } from './types.js';

class SlashCommandRegistry {
	#commands = new SvelteMap<string, RegisteredSlashCommand>();

	/** Add commands. Call the returned function to remove exactly this batch. */
	register(commands: SlashCommand[], source: CommandSource = 'core'): () => void {
		const names: string[] = [];
		for (const command of commands) {
			this.#commands.set(command.name, { ...command, source });
			names.push(command.name);
		}
		return () => {
			for (const name of names) this.#commands.delete(name);
		};
	}

	get all(): RegisteredSlashCommand[] {
		return [...this.#commands.values()];
	}

	#byAlias = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const aliases = new Map<string, RegisteredSlashCommand>();
		for (const command of this.#commands.values()) {
			for (const alias of command.aliases ?? []) aliases.set(alias, command);
		}
		return aliases;
	});

	/** Look one name up, following aliases. */
	resolve(name: string): RegisteredSlashCommand | undefined {
		return this.#commands.get(name) ?? this.#byAlias.get(name);
	}

	/** Commands whose name or alias starts with `prefix`. Sorted alphabetically. */
	matching(prefix: string): RegisteredSlashCommand[] {
		const query = prefix.toLowerCase();
		return this.all
			.filter(
				(command) =>
					command.name.startsWith(query) ||
					(command.aliases ?? []).some((alias) => alias.startsWith(query))
			)
			.sort((a, b) => a.name.localeCompare(b.name));
	}
}

/**
 * Every command reachable from the composer.
 *
 * Client-side only for now. Server-backed weaver commands will register here
 * too, wrapping a run endpoint — the composer will not know the difference.
 */
export const slashCommands = new SlashCommandRegistry();
