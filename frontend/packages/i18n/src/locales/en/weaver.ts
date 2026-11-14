/** Help text in the weaver editor: autocomplete, hover cards and diagnostics. */
export const weaver = {
	keywords: {
		if: 'Conditional block. Body renders when `<expr>` is truthy.\n\nInside an array or object literal it stands in for items, contributing the ones its taken branch holds.',
		elif: 'Additional branch in an `if` block.',
		else: 'Fallback branch in an `if` block.',
		endif: 'Close an `if` block.',
		foreach:
			'Iterate `<array>`, binding each element to `<name>` in a temporary scope.\n\nInside an array or object literal it stands in for items, contributing one per iteration.',
		endforeach: 'Close a `foreach` block.',
		in: 'Separates the loop binding from the iterable.',
		break: 'Immediately break out of the innermost `foreach`.',
		continue: 'Skip to the next iteration of the innermost `foreach`.',
		return: 'End evaluation immediately and return the value of `<expr>`.',
		stop: 'End evaluation immediately, keeping the text rendered so far.'
	},
	kinds: {
		namespace: 'namespace',
		group: 'group',
		processor: 'processor',
		command: 'command',
		keyword: 'keyword',
		entry: 'entry',
		trigger: 'trigger',
		declaredHere: 'declared here',
		parameter: 'parameter',
		loopBinding: 'loop binding'
	},
	namespaceDoc: 'A state namespace this document uses.',
	holds: (list: string, more: boolean) => `Holds ${list}${more ? ', and more' : ''}.`,
	inEntry: (detail: string) => `In ${detail}.`,
	triggerDoc: "Render another entry's text in place.",
	resolvesTo: (value: string) => `Resolves to \`${value}\` for the loaded context.`,
	noValue: 'No loaded value for this variable in the current context.',
	writtenByCommand: 'Written by a command in this body.',
	parameterOf: (proc: string) => `A parameter of \`${proc}\`.`,
	parameterDefault: (proc: string, fallback: string) =>
		`A parameter of \`${proc}\`. Defaults to \`${fallback}\`.`,
	eachElement: (iterable: string) => `Each element of \`${iterable}\`.`,
	hint: (message: string, hint: string) => `${message}\n\nHint: ${hint}`
} as const;
