import { showToast } from '#lib/stores/errorToast.svelte.js';
import { extractError } from '#lib/types/error.js';
import { parseArgs, parseInput } from './parse.js';
import { slashCommands } from './registry.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

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
			title: t.commands.unknown,
			message: t.commands.noSuchCommand(name),
			variant: 'error'
		});
		return;
	}

	if (command.enabled && !command.enabled()) {
		showToast({
			title: t.commands.unavailableTitle(command.name),
			message: command.unavailable ?? t.commands.notAvailable,
			variant: 'warning'
		});
		return;
	}

	try {
		await command.run(command.raw ? { named: {}, rest } : parseArgs(rest));
	} catch (error) {
		if (extractError(error).code === 'command.cancelled') return;
		showToast({
			title: t.commands.failed(command.name),
			message: error instanceof Error ? error.message : String(error),
			variant: 'error'
		});
	}
}
