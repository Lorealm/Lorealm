import { showToast } from '$lib/stores/errorToast.svelte.js';
import { chordsFromEvent } from './chord.js';
import { keybindConfig } from './config.svelte.js';
import { inputEnvironment } from './environment.svelte.js';
import { layers } from './layers.svelte.js';
import { commandRegistry } from './registry.svelte.js';
import type { CommandContext, CommandResult, RegisteredCommand } from './types.js';

/**
 * Input types that neither hold text nor steer by keyboard, so a keystroke over
 * one of them is fair game. `radio` and `range` are deliberately absent as they
 * answer to the arrow keys themselves.
 */
const PASSIVE_INPUTS = new Set([
	'button',
	'checkbox',
	'color',
	'file',
	'hidden',
	'image',
	'reset',
	'submit'
]);

/**
 * Whether the keystroke landed on a control that is already using it such as a text
 * field the user is composing in, or a widget that steers by keyboard.
 */
function isEditable(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) return false;
	if (target.isContentEditable) return true;

	const tag = target.tagName;
	if (tag === 'TEXTAREA' || tag === 'SELECT') return true;
	if (tag === 'INPUT') return !PASSIVE_INPUTS.has((target as HTMLInputElement).type);

	return target.closest('.cm-editor') !== null;
}

/**
 * Turns keystrokes into commands.
 */
class KeybindManager {
	suspended = $state(false);

	get enabled(): boolean {
		switch (keybindConfig.mode) {
			case 'off':
				return false;
			case 'on':
				return true;
			default:
				return !inputEnvironment.touchOnly;
		}
	}

	install(): () => void {
		const onKeyDown = (event: KeyboardEvent) => this.handle(event);
		window.addEventListener('keydown', onKeyDown);
		return () => window.removeEventListener('keydown', onKeyDown);
	}

	handle(event: KeyboardEvent): void {
		if (!this.enabled || this.suspended) return;

		// CodeMirror and the component-level handlers preventDefault what they
		// consume but never stop propagation, so every keystroke in the app reaches
		// this. This is how we stay off the keys they have already claimed.
		if (event.defaultPrevented) return;

		const chords = chordsFromEvent(event);
		if (chords.length === 0) return;

		const editable = isEditable(event.target);

		for (const layer of layers.chainFor(event.target)) {
			for (const chord of chords) {
				for (const command of commandRegistry.lookup(layer.id, chord)) {
					if (editable && !command.allowInEditable) continue;

					const context: CommandContext = { event, layer: layer.id, data: layer.data };
					if (command.enabled?.(context) === false) continue;
					if (this.#run(command, context) === false) continue;

					event.preventDefault();
					return;
				}
			}
			// A modal owns the screen: nothing behind it gets a turn.
			if (layer.blocking) return;
		}
	}

	#run(command: RegisteredCommand, context: CommandContext): CommandResult {
		try {
			return command.run(context);
		} catch (error) {
			showToast({
				title: `${command.title} failed`,
				message: error instanceof Error ? error.message : String(error),
				variant: 'error'
			});
			return undefined;
		}
	}
}

export const keybinds = new KeybindManager();
