export type Construct = 'var' | 'proc' | 'cmd' | 'tag' | 'doc' | null;

type Frame = Exclude<Construct, null> | 'array' | 'object';

const OPENERS: { k: Frame; open: string; close: string }[] = [
	{ k: 'var', open: '{{', close: '}}' },
	{ k: 'tag', open: '{#', close: '#}' },
	{ k: 'doc', open: '[[', close: ']]' },
	{ k: 'proc', open: '@[', close: ']' },
	{ k: 'cmd', open: '$[', close: ']' },
	{ k: 'array', open: '[', close: ']' },
	{ k: 'object', open: '{', close: '}' }
];

/**
 * Constructs whose bodies are expressions
 */
const EXPR_BODY = new Set<Frame>(['var', 'tag', 'proc', 'cmd', 'array', 'object']);

/** Frames that are containers rather than constructs a caller completes in. */
const ANONYMOUS = new Set<Frame>(['array', 'object']);

export function activeConstruct(before: string): Construct {
	const stack: Frame[] = [];

	let i = 0;
	scan: while (i < before.length) {
		const top = stack[stack.length - 1];

		if (top) {
			const close = OPENERS.find((o) => o.k === top)!.close;
			if (before.startsWith(close, i)) {
				stack.pop();
				i += close.length;
				continue;
			}
			if (before[i] === '"') {
				// Unterminated: everything left is inside the string.
				const end = findStringEnd(before, i);
				if (end === -1) return report(stack);
				i = end;
				continue;
			}
		}

		const inExpr = !!top && EXPR_BODY.has(top);
		for (const o of OPENERS) {
			if (ANONYMOUS.has(o.k) && !inExpr) continue;
			if (before.startsWith(o.open, i)) {
				stack.push(o.k);
				i += o.open.length;
				continue scan;
			}
		}
		i++;
	}

	return report(stack);
}

/** Index just past the string literal starting at `start`, or -1 if unclosed. */
function findStringEnd(text: string, start: number): number {
	for (let i = start + 1; i < text.length; i++) {
		if (text[i] === '\\') i++;
		else if (text[i] === '"') return i + 1;
	}
	return -1;
}

/** The innermost frame that names a construct callers care about. */
function report(stack: Frame[]): Construct {
	for (let i = stack.length - 1; i >= 0; i--) {
		if (!ANONYMOUS.has(stack[i])) return stack[i] as Construct;
	}
	return null;
}
