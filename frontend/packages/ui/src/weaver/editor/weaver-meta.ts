export interface DocEntry {
	signature: string;
	doc: string;
}

export const keywords: Record<string, DocEntry> = {
	if: {
		signature: '{# if <expr> #} ... {# endif #}',
		doc: 'Conditional block. Body renders when `<expr>` is truthy.\n\nInside an array or object literal it stands in for items, contributing the ones its taken branch holds.'
	},
	elif: { signature: '{# elif <expr> #}', doc: 'Additional branch in an `if` block.' },
	else: { signature: '{# else #}', doc: 'Fallback branch in an `if` block.' },
	endif: { signature: '{# endif #}', doc: 'Close an `if` block.' },
	foreach: {
		signature: '{# foreach <name> in <array> #} ... {# endforeach #}',
		doc: 'Iterate `<array>`, binding each element to `<name>` in a temporary scope.\n\nInside an array or object literal it stands in for items, contributing one per iteration.'
	},
	endforeach: { signature: '{# endforeach #}', doc: 'Close a `foreach` block.' },
	in: {
		signature: 'foreach <name> in <array>',
		doc: 'Separates the loop binding from the iterable.'
	},
	break: {
		signature: '{# break #}',
		doc: 'Immediately break out of the innermost `foreach`.'
	},
	continue: {
		signature: '{# continue #}',
		doc: 'Skip to the next iteration of the innermost `foreach`.'
	},
	return: {
		signature: '{# return <expr> #}',
		doc: 'End evaluation immediately and return the value of `<expr>`.'
	},
	stop: {
		signature: '{# stop #}',
		doc: 'End evaluation immediately, keeping the text rendered so far.'
	}
};
