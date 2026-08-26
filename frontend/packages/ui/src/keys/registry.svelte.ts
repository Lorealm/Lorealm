import { SvelteMap } from 'svelte/reactivity';
import { parseChord, type Chord } from './chord.js';
import { keybindConfig } from './config.svelte.js';
import type { Command, CommandSource, RegisteredCommand } from './types.js';

/** Two (or more) commands on one layer answering to the same chord. */
export interface Conflict {
	layer: string;
	chord: Chord;
	commands: RegisteredCommand[];
}

interface Index {
	byLayer: Map<string, Map<Chord, RegisteredCommand[]>>;
	conflicts: Conflict[];
}

function sameSource(a: CommandSource, b: CommandSource): boolean {
	if (a === 'core' || b === 'core') return a === b;
	return a.plugin === b.plugin;
}

class CommandRegistry {
	#commands = new SvelteMap<string, RegisteredCommand>();

	/** Add commands. Call the returned function to remove exactly this batch. */
	register(commands: Command[], source: CommandSource = 'core'): () => void {
		const ids: string[] = [];
		for (const command of commands) {
			this.#commands.set(command.id, { ...command, source });
			ids.push(command.id);
		}
		return () => {
			for (const id of ids) this.#commands.delete(id);
		};
	}

	/** Remove everything one plugin contributed. */
	unregisterSource(source: CommandSource): void {
		for (const [id, command] of this.#commands) {
			if (sameSource(command.source, source)) this.#commands.delete(id);
		}
	}

	get all(): RegisteredCommand[] {
		return [...this.#commands.values()];
	}

	get(id: string): RegisteredCommand | undefined {
		return this.#commands.get(id);
	}

	/** The chords actually in effect for a command: the user's override, else its defaults. */
	keysFor(command: Command): Chord[] {
		const written = keybindConfig.overrides[command.id] ?? command.keys;
		return written.map(parseChord).filter((chord): chord is Chord => chord !== null);
	}

	#index: Index = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const byLayer = new Map<string, Map<Chord, RegisteredCommand[]>>();

		for (const command of this.#commands.values()) {
			let layer = byLayer.get(command.layer);
			// eslint-disable-next-line svelte/prefer-svelte-reactivity
			if (!layer) byLayer.set(command.layer, (layer = new Map()));

			for (const chord of this.keysFor(command)) {
				const bound = layer.get(chord);
				if (bound) bound.push(command);
				else layer.set(chord, [command]);
			}
		}

		const conflicts: Conflict[] = [];
		for (const [layerId, chords] of byLayer) {
			for (const [chord, commands] of chords) {
				if (commands.length > 1) conflicts.push({ layer: layerId, chord, commands });
			}
		}

		return { byLayer, conflicts };
	});

	/** Candidates for one chord on one layer, in registration order. */
	lookup(layer: string, chord: Chord): RegisteredCommand[] {
		return this.#index.byLayer.get(layer)?.get(chord) ?? [];
	}

	get conflicts(): Conflict[] {
		return this.#index.conflicts;
	}

	/** The other commands that would answer to this chord on the same layer. */
	conflictsWith(command: Command, chord: Chord): RegisteredCommand[] {
		return this.lookup(command.layer, chord).filter((other) => other.id !== command.id);
	}
}

export const commandRegistry = new CommandRegistry();
