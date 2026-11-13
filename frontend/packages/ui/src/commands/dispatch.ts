import { showToast } from '#lib/stores/errorToast.svelte.js';
import { extractError } from '#lib/types/error.js';
import { parseArgs, parseInput } from './parse.js';
import { slashCommands } from './registry.svelte.js';

/**
 * Run one line of composer input.
 */
export async function dispatch(raw: string): Promise<void> {
	const input = raw.trim();
	if (!input) return;

	const parsed = parseInput(input);
	if (parsed.kind === 'text') return invoke('send', parsed.text);
	return invoke(parsed.name, parsed.rest);
}

async function invoke(name: string, rest: string): Promise<void> {
	const command = slashCommands.resolve(name);
	if (!command) {
		showToast({
			title: 'Unknown command',
			message: `There is no /${name} command.`,
			variant: 'error'
		});
		return;
	}

	if (command.enabled && !command.enabled()) {
		showToast({
			title: `/${command.name} is unavailable`,
			message: command.unavailable ?? 'Not available right now.',
			variant: 'warning'
		});
		return;
	}

	try {
		await command.run(command.raw ? { named: {}, rest } : parseArgs(rest));
	} catch (error) {
		if (extractError(error).code === 'command.cancelled') return;
		showToast({
			title: `/${command.name} failed`,
			message: error instanceof Error ? error.message : String(error),
			variant: 'error'
		});
	}
}
