import type { CommandArgs } from './types.js';

export type ParsedInput =
	| { kind: 'text'; text: string }
	| { kind: 'command'; name: string; rest: string };

const NAMED_ARG = /^([A-Za-z_][\w-]*)=/;

/**
 * Split one line of composer input into a command invocation or plain text.
 */
export function parseInput(raw: string): ParsedInput {
	if (!raw.startsWith('/')) return { kind: 'text', text: raw };
	if (raw.startsWith('//')) return { kind: 'text', text: raw.slice(1) };
	if (raw === '/' || /^\/\s/.test(raw)) return { kind: 'text', text: raw };

	const body = raw.slice(1);
	const space = body.search(/\s/);
	if (space === -1) return { kind: 'command', name: body, rest: '' };
	return { kind: 'command', name: body.slice(0, space), rest: body.slice(space + 1) };
}

/**
 * Peel leading `key=value` pairs off a command's argument text.
 *
 * Values may be quoted with `"` or `'` to hold spaces, and a backslash
 * escapes the next character inside quotes. Parsing stops at the first token
 * that is not a pair, so `/send x=1 is fine` is unambiguous only for commands
 * that opt out via {@link import('./types.js').SlashCommand.raw}.
 */
export function parseArgs(input: string): CommandArgs {
	const named: Record<string, string> = {};
	let at = 0;

	for (;;) {
		while (at < input.length && /\s/.test(input[at])) at++;
		const match = NAMED_ARG.exec(input.slice(at));
		if (!match) break;

		let cursor = at + match[0].length;
		const quote = input[cursor];
		let value: string;

		if (quote === '"' || quote === "'") {
			cursor++;
			let out = '';
			while (cursor < input.length && input[cursor] !== quote) {
				if (input[cursor] === '\\' && cursor + 1 < input.length) {
					out += input[cursor + 1];
					cursor += 2;
				} else {
					out += input[cursor++];
				}
			}
			cursor++; // past the closing quote; an unterminated one just ends here
			value = out;
		} else {
			const start = cursor;
			while (cursor < input.length && !/\s/.test(input[cursor])) cursor++;
			value = input.slice(start, cursor);
		}

		named[match[1]] = value;
		at = cursor;
	}

	return { named, rest: input.slice(at).trim() };
}
