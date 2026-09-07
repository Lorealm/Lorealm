import { untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { CommandSource } from '$lib/keys/types.js';
import type { RegisteredSlashCommand, SlashCommand } from './types.js';

class SlashCommandRegistry {
	/**
	 * Every registration for a name, oldest first; the last one answers.
	 */
	#stacks = new SvelteMap<string, RegisteredSlashCommand[]>();

	/**
	 * Add commands. Call the returned function to remove exactly this batch,
	 * whenever and in whatever order.
	 */
	register(commands: SlashCommand[], source: CommandSource = 'core'): () => void {
		const added: RegisteredSlashCommand[] = [];

		untrack(() => {
			for (const command of commands) {
				const entry = { ...command, source };
				added.push(entry);
				// Replaced rather than mutated: a push would not notify readers.
				this.#stacks.set(command.name, [...(this.#stacks.get(command.name) ?? []), entry]);
			}
		});

		return () =>
			untrack(() => {
				for (const entry of added) {
					const stack = this.#stacks.get(entry.name);
					if (!stack) continue;
					const remaining = stack.filter((held) => held !== entry);
					if (remaining.length === stack.length) continue;
					if (remaining.length > 0) this.#stacks.set(entry.name, remaining);
					else this.#stacks.delete(entry.name);
				}
			});
	}

	/** The command answering to each name. */
	get all(): RegisteredSlashCommand[] {
		return [...this.#stacks.values()].map((stack) => stack[stack.length - 1]);
	}

	#byAlias = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const aliases = new Map<string, RegisteredSlashCommand>();
		for (const command of this.all) {
			for (const alias of command.aliases ?? []) aliases.set(alias, command);
		}
		return aliases;
	});

	/** Look one name up, following aliases. */
	resolve(name: string): RegisteredSlashCommand | undefined {
		const stack = this.#stacks.get(name);
		return stack ? stack[stack.length - 1] : this.#byAlias.get(name);
	}

	/** Commands whose name or alias starts with `prefix`, alphabetically. */
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

	/** Whether `name` currently shadows another registration. */
	shadows(name: string): boolean {
		return (this.#stacks.get(name)?.length ?? 0) > 1;
	}
}

/**
 * Every command reachable from the composer.
 */
export const slashCommands = new SlashCommandRegistry();
