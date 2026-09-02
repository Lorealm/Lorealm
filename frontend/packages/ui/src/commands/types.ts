import type { CommandSource } from '$lib/keys/types.js';

/**
 * A command's arguments: leading `key=value` pairs, then whatever text
 * followed them.
 *
 * Named arguments must come first. The first token
 * that is not a `key=value` pair begins {@link CommandArgs.rest}.
 */
export interface CommandArgs {
	named: Record<string, string>;
	/** Everything after the last named argument, trimmed. */
	rest: string;
}

export interface SlashParam {
	name: string;
	description: string;
	required: boolean;
}

/**
 * A command reachable by typing `/name` in the composer.
 */
export interface SlashCommand {
	name: string;
	aliases?: string[];
	description: string;
	/** Named parameters, shown in the autocomplete hint. */
	params?: SlashParam[];
	/** What trailing text means, e.g. `message`. Omit when unused. */
	restLabel?: string;
	/** Hand `run` the raw remainder rather than parsing `key=value` out of it. */
	raw?: boolean;
	/** Checked before running. A false answer is reported, not swallowed. */
	enabled?(): boolean;
	/** Why it is unavailable, when {@link enabled} says no. */
	unavailable?: string;
	run(args: CommandArgs): void | Promise<void>;
}

export type RegisteredSlashCommand = SlashCommand & { source: CommandSource };

/** The human-readable signature line for autocomplete. f.ex: `/send <message>` */
export function usage(command: SlashCommand): string {
	const parts = [`/${command.name}`];
	for (const param of command.params ?? []) {
		parts.push(param.required ? `${param.name}=` : `[${param.name}=]`);
	}
	if (command.restLabel) parts.push(`<${command.restLabel}>`);
	return parts.join(' ');
}
