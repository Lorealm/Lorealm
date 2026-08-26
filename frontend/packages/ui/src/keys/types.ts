import type { Chord } from './chord.js';

/**
 * Where a command listens.
 *
 * - `global` — always reachable.
 * - `view` — only while the router is on that view; its layer id is the
 *   {@link import('$lib/navigation/routes.js').ViewName}.
 * - `module` — only while a matching layer is focused. Beats both of the above.
 */
export type LayerScope = 'global' | 'view' | 'module';

/** Who contributed a command. */
export type CommandSource = 'core' | { plugin: string };

/** Everything a command handler is handed. */
export interface CommandContext {
	event: KeyboardEvent;
	/** The layer whose binding matched. */
	layer: string;
	data: unknown;
}

/**
 * A result wrapper carrying an arbitrary function. `false` means the command declined the keystroke.
 */
export type CommandResult = void | boolean;

export interface Command {
	id: string;
	title: string;
	description?: string;
	group: string;
	scope: LayerScope;
	layer: string;
	/** Default chords, in {@link parseChord} syntax. */
	keys: string[];
	run(context: CommandContext): CommandResult;
	/** Checked before running. Return `false` and dispatch carries on outwards. */
	enabled?(context: CommandContext): boolean;
	/**
	 * Run even while the user is typing in an input, textarea, or CodeMirror
	 * editor.
	 */
	allowInEditable?: boolean;
	/** Keep out of the settings catalogue. */
	hidden?: boolean;
	source?: CommandSource;
}

export type RegisteredCommand = Command & { source: CommandSource };

export interface LayerOptions {
	id: string;
	/**
	 * Anchors the layer to the DOM: it is active while focus is inside this
	 * element, and nested layers beat it. Set for you by `keyLayer`.
	 */
  element?: HTMLElement;
	active?: () => boolean;
	/** Dispatch stops here rather than falling through to the layers behind it. */
	blocking?: boolean;
	/** Handed to matching commands as {@link CommandContext.data}. */
	data?: unknown;
}

export interface Layer extends LayerOptions {
	/** Registration order */
	readonly seq: number;
}

export type { Chord };
