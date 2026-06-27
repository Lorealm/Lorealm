import { StreamLanguage, type StreamParser } from '@codemirror/language';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

type Frame =
	| { k: 'var' }
	| { k: 'tag' }
	| { k: 'doc' }
	| { k: 'trigger' }
	| { k: 'proc'; named: boolean }
	| { k: 'cmd'; named: boolean };

interface State {
	stack: Frame[];
}

const IDENT = /^[A-Za-z_]\w*/;
const DOTTED = /^[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*/; // e.g. core.weaver.rng
const KEYWORD = /^(?:if|elif|else|endif|foreach|endforeach|in)\b/;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function exprAtom(stream: any, state: State): string | null {
	// Nested constructs
	if (stream.match('{{')) {
		state.stack.push({ k: 'var' });
		return 'delimiter';
	}
	if (stream.match('@[')) {
		state.stack.push({ k: 'proc', named: false });
		return 'bracket';
	}
	if (stream.match('$[')) {
		state.stack.push({ k: 'cmd', named: false });
		return 'bracket';
	}
	if (stream.match('[[')) {
		state.stack.push({ k: 'doc' });
		return 'bracket';
	}
	if (stream.match(/^<\/?trigger\b/)) {
		state.stack.push({ k: 'trigger' });
		return 'tagName';
	}
	// Literals
	if (stream.match(/^"(?:[^"\\]|\\.)*"/)) return 'string';
	if (stream.match(/^-?\d+(?:\.\d+)?/)) return 'number';
	if (stream.match(/^(?:true|false)\b/)) return 'bool';
	if (stream.match(/^none\b/)) return 'none';
	// Operators (multi-char first)
	if (stream.match(/^(?:==|!=|<=|>=|&&|\|\|)/)) return 'operator';
	if (stream.match(/^[+\-*/<>!]/)) return 'operator';
	// Punctuation
	if (stream.match(/^[(),]/)) return 'punctuation';
	if (stream.match(/^:/)) return 'punctuation';
	// Bare identifier
	if (stream.match(IDENT)) return 'varName';
	return null;
}

const parser: StreamParser<State> = {
	startState: () => ({ stack: [] }),

	// Deep copy: the default copyState only slices the array, leaving frame
	// objects shared. We mutate `named` in place, so clone frames too.
	copyState: (s) => ({ stack: s.stack.map((f) => ({ ...f })) }),

	token(stream, state) {
		const top = state.stack[state.stack.length - 1];

		// ---- Plain text mode ----
		if (!top) {
			if (stream.match('{{')) {
				state.stack.push({ k: 'var' });
				return 'delimiter';
			}
			if (stream.match('{#')) {
				state.stack.push({ k: 'tag' });
				return 'delimiter';
			}
			if (stream.match('@[')) {
				state.stack.push({ k: 'proc', named: false });
				return 'bracket';
			}
			if (stream.match('$[')) {
				state.stack.push({ k: 'cmd', named: false });
				return 'bracket';
			}
			if (stream.match('[[')) {
				state.stack.push({ k: 'doc' });
				return 'bracket';
			}
			if (stream.match(/^<\/?trigger\b/)) {
				state.stack.push({ k: 'trigger' });
				return 'tagName';
			}
			if (stream.match(/^[^{@$[<]+/)) return null;
			stream.next();
			return null;
		}

		if (stream.eatSpace()) return null;

		switch (top.k) {
			// {{ scope:name }}
			case 'var': {
				if (stream.match('}}')) {
					state.stack.pop();
					return 'delimiter';
				}
				if (stream.match(/^[A-Za-z_]\w*(?=\s*:)/)) return 'scope';
				if (stream.match(':')) return 'punctuation';
				if (stream.match(IDENT)) return 'varName';
				stream.next();
				return null;
			}

			// {# if ... #} / {# foreach x in y #}
			case 'tag': {
				if (stream.match('#}')) {
					state.stack.pop();
					return 'delimiter';
				}
				if (stream.match(KEYWORD)) return 'keyword';
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// @[ namespace.name(key: value) ]
			case 'proc': {
				if (stream.match(']')) {
					state.stack.pop();
					return 'bracket';
				}
				if (!top.named && stream.match(DOTTED)) {
					top.named = true;
					return 'func';
				}
				if (stream.match(/^[A-Za-z_]\w*(?=\s*:)/)) return 'propName'; // arg key
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// $[ name(arg1, arg2) ]
			case 'cmd': {
				if (stream.match(']')) {
					state.stack.pop();
					return 'bracket';
				}
				if (!top.named && stream.match(DOTTED)) {
					top.named = true;
					return 'func';
				}
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// [[ document_id ]]
			case 'doc': {
				if (stream.match(']]')) {
					state.stack.pop();
					return 'bracket';
				}
				if (stream.match(/^[A-Za-z0-9_]+/)) return 'docId';
				stream.next();
				return null;
			}

			// <trigger id="...">
			case 'trigger': {
				if (stream.match('/>') || stream.match('>')) {
					state.stack.pop();
					return 'tagName';
				}
				if (stream.match(/^[A-Za-z_][\w-]*(?=\s*=)/)) return 'attrName';
				if (stream.match('=')) return 'operator';
				if (stream.match(/^"(?:[^"\\]|\\.)*"/)) return 'string';
				if (stream.match(IDENT)) return 'varName';
				stream.next();
				return null;
			}
		}
	},

	indent(state, textAfter, cx) {
		const depth = state.stack.length;
		if (depth === 0) return null; // top-level prose: leave as typed
		const dedent = /^[)\]}]/.test(textAfter) ? 1 : 0;
		return Math.max(0, depth - dedent) * cx.unit;
	},

	languageData: {
		name: 'weaver',
		indentOnInput: /^\s*[)\]}]$/
	},

	tokenTable: {
		delimiter: t.brace,
		bracket: t.squareBracket,
		keyword: t.controlKeyword,
		scope: t.namespace,
		varName: t.variableName,
		func: t.function(t.variableName),
		propName: t.propertyName,
		string: t.string,
		number: t.number,
		bool: t.bool,
		none: t.null,
		operator: t.operator,
		punctuation: t.punctuation,
		tagName: t.tagName,
		attrName: t.attributeName,
		docId: t.labelName
	}
};

export const weaverLanguage = StreamLanguage.define(parser);

const weaverHighlight = HighlightStyle.define([
	{ tag: t.brace, color: '#7c8a9e', fontWeight: '600' }, // {{ }} {# #}
	{ tag: t.squareBracket, color: '#c98a3a', fontWeight: '600' }, // @[ ] $[ ] [[ ]]
	{ tag: t.controlKeyword, color: '#c678dd', fontWeight: '600' },
	{ tag: t.namespace, color: '#56b6c2' }, // scope before colon
	{ tag: t.variableName, color: '#e5c07b' },
	{ tag: t.function(t.variableName), color: '#61afef' }, // processor/command name
	{ tag: t.propertyName, color: '#98c379' }, // arg keys
	{ tag: t.string, color: '#98c379' },
	{ tag: t.number, color: '#d19a66' },
	{ tag: t.bool, color: '#d19a66' },
	{ tag: t.null, color: '#d19a66' },
	{ tag: t.operator, color: '#56b6c2' },
	{ tag: t.punctuation, color: '#828a99' },
	{ tag: t.tagName, color: '#e06c75', fontWeight: '600' }, // <trigger>
	{ tag: t.attributeName, color: '#d19a66' }, // id=
	{ tag: t.labelName, color: '#e5c07b', textDecoration: 'underline' } // [[doc]]
]);

export const weaverSyntax = [weaverLanguage, syntaxHighlighting(weaverHighlight)];
