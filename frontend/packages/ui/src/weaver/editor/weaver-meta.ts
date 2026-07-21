export interface DocEntry {
	signature: string;
	doc: string;
}

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
