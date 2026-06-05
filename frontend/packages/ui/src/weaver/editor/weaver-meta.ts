export interface DocEntry {
	signature: string;
	doc: string;
}

// Processors: @[namespace.name(...)] .
export const processors: Record<string, DocEntry> = {
	'text.upper': { signature: 'text.upper(text: String) -> String', doc: 'Uppercase `text`.' },
	'text.lower': { signature: 'text.lower(text: String) -> String', doc: 'Lowercase `text`.' },
	'text.capitalize': {
		signature: 'text.capitalize(text: String) -> String',
		doc: 'Capitalize the first character of `text`.'
	},
	'text.length': {
		signature: 'text.length(text: String) -> Number',
		doc: 'Character count of `text`.'
	},
	'text.replace': {
		signature: 'text.replace(text: String, from: String, to: String) -> String',
		doc: 'Replace occurrences of `from` with `to` in `text`.'
	},
	'math.add': { signature: 'math.add(a: Number, b: Number) -> Number', doc: 'Returns `a + b`.' },
	'math.sub': { signature: 'math.sub(a: Number, b: Number) -> Number', doc: 'Returns `a - b`.' },
	'math.clamp': {
		signature: 'math.clamp(value: Number, min: Number, max: Number) -> Number',
		doc: 'Constrain `value` to the inclusive range `[min, max]`.'
	},
	'array.contains': {
		signature: 'array.contains(items: Array, value: Value) -> Bool',
		doc: 'True when `items` contains `value`.'
	},
	'array.length': {
		signature: 'array.length(items: Array) -> Number',
		doc: 'Element count of `items`.'
	},
	'core.rng': {
		signature: 'core.rng(min: Number, max: Number) -> Number',
		doc: 'Random integer in `[min, max]`. Not pure — re-rolls each evaluation.'
	}
};

// Commands: $[name(...)].
export const commands: Record<string, DocEntry> = {
	set_var: {
		signature: 'set_var(key: String, value: Value)',
		doc: 'Write `value` into a read-write namespace. `key` is `scope:name`, e.g. `state:visited`.'
	},
	get_var: { signature: 'get_var(key: String) -> Value', doc: 'Read a variable by `scope:name`.' },
	inc_var: {
		signature: 'inc_var(key: String, by: Number = 1)',
		doc: 'Increment a numeric variable in place.'
	},
	push_var: {
		signature: 'push_var(key: String, value: Value)',
		doc: 'Append `value` to an array variable.'
	},
	default_var: {
		signature: 'default_var(key: String, value: Value)',
		doc: 'Set `key` only if it is currently undefined.'
	},
	is_active: {
		signature: 'is_active(entry_id: String) -> Bool',
		doc: 'True if the named entry is active this turn.'
	}
};

// Control-flow keywords.
export const keywords: Record<string, DocEntry> = {
	if: {
		signature: '{# if <expr> #} ... {# endif #}',
		doc: 'Conditional block. Body renders when `<expr>` is truthy.'
	},
	elif: { signature: '{# elif <expr> #}', doc: 'Additional branch in an `if` block.' },
	else: { signature: '{# else #}', doc: 'Fallback branch in an `if` block.' },
	endif: { signature: '{# endif #}', doc: 'Close an `if` block.' },
	foreach: {
		signature: '{# foreach <name> in <array> #} ... {# endforeach #}',
		doc: 'Iterate `<array>`, binding each element to `<name>` in a temporary scope.'
	},
	endforeach: { signature: '{# endforeach #}', doc: 'Close a `foreach` block.' },
	in: {
		signature: 'foreach <name> in <array>',
		doc: 'Separates the loop binding from the iterable.'
	}
};
