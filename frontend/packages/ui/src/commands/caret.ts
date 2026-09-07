/**
 * What the caret is sitting in
 */
export type CaretContext =
	| { kind: 'none' }
	/** A command name being typed. `from`/`to` span the whole `/name`. */
	| { kind: 'command'; prefix: string; from: number; to: number }
	/** A `key=value` argument. `from`/`to` span the value, not the `key=`. */
	| {
			kind: 'argument';
			command: string;
			name: string;
			value: string;
			from: number;
			to: number;
	  };

/** `/na`: a command name being typed, with nothing after it. */
const COMMAND_NAME = /^\/([A-Za-z_-]*)$/;
/** `… profile=hai`: an argument value ending at the caret. */
const ARGUMENT = /(?:^|\s)([A-Za-z_][\w-]*)=([^\s]*)$/;
/** `/name `: the command an argument belongs to. */
const INVOCATION = /^\/([A-Za-z_-]+)\s/;

export function caretContext(text: string, caret: number): CaretContext {
	const before = text.slice(0, caret);

	// A bare name, with nothing typed after it anywhere in the box.
	const naming = COMMAND_NAME.exec(before);
	if (naming && caret === text.length) {
		return { kind: 'command', prefix: naming[1], from: 0, to: caret };
	}

	const invocation = INVOCATION.exec(before);
	const argument = ARGUMENT.exec(before);
	if (!invocation || !argument) return { kind: 'none' };

	return {
		kind: 'argument',
		command: invocation[1],
		name: argument[1],
		value: argument[2],
		from: caret - argument[2].length,
		to: caret
	};
}
