import { StreamLanguage, type StreamParser } from '@codemirror/language';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

type Frame =
	| { k: 'var'; opened?: boolean }
	| { k: 'tag'; opened?: boolean }
	| { k: 'doc' }
	| { k: 'trigger' }
	| { k: 'array' }
	| { k: 'object'; key: boolean }
	| { k: 'paren' }
	| { k: 'params' }
	| { k: 'proc'; named: boolean; args?: boolean; opened?: boolean }
	| { k: 'cmd'; named: boolean; args?: boolean; opened?: boolean };

/**
 * Where a module's top level is. Absent outside the module dialect.
 */
type Phase = 'top' | 'export' | 'procName' | 'sig' | 'returns' | 'body';

interface State {
	stack: Frame[];
	phase?: Phase;
}

export const DELIMITER_COLOR = '#7c8a9e';
const TRIM_COLOR = '#d782a8';

const IDENT = /^[A-Za-z_]\w*/;
const DOTTED = /^[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*/; // e.g. core.weaver.rng
const KEYWORD =
	/^(?:if|elif|else|endif|foreach|endforeach|endproc|in|break|continue|return|stop)\b/;
/** A parameter's declared type: the same vocabulary as `#inputs`. */
const PARAM_TYPE = /^(?:Ref\s*<\s*[A-Za-z_]\w*\s*>|(?:string|number|bool|enum|range|span)\b)/;
/** A name before a colon: the scope of a reference, or a call's argument key. */
const NAMED = /^[A-Za-z_]\w*(?=\s*:)/;
/** `Name(` — an enum variant. The paren has to be glued to the name. */
const VARIANT = /^[A-Za-z_]\w*(?=\()/;
/** A field suffix, glued to the value it reads: `.name`. */
const FIELD = /^\.[A-Za-z_]\w*/;

const STRING = /^"(?:[^"\\]|\\.)*"/;
const RAW_STRING = /^r(#*)"[\s\S]*?"\1/;
const NUMBER = /^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/;
const COMMENT = /^\/\/.*/;

const OPEN_VAR = /^\{\{/;
const CLOSE_VAR = /^\}\}/;
const OPEN_TAG = /^\{#/;
const CLOSE_TAG = /^#\}/;
const OPEN_PROC = /^@\[/;
const OPEN_CMD = /^\$\[/;
const CLOSE_CALL = /^\]/; // shared by `@[ … ]` and `$[ … ]`

const TRIM_L = /^-(?!\d)/;
const TRIM_VAR = /^-(?=\}\})/;
const TRIM_TAG = /^-(?=#\})/;
const TRIM_CALL = /^-(?=\])/;

function trimmable(frame: Frame): frame is Extract<Frame, { opened?: boolean }> {
	return frame.k === 'var' || frame.k === 'tag' || frame.k === 'proc' || frame.k === 'cmd';
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function exprAtom(stream: any, state: State): string | null {
	// A comment runs to the end of the line, anywhere whitespace is allowed.
	if (stream.match(COMMENT)) return 'comment';
	// Nested constructs
	if (stream.match(OPEN_VAR)) {
		state.stack.push({ k: 'var', opened: true });
		return 'delimiter';
	}
	// A loop or a conditional may stand in for items of a collection.
	if (stream.match(OPEN_TAG)) {
		state.stack.push({ k: 'tag', opened: true });
		return 'delimiter';
	}
	if (stream.match(OPEN_PROC)) {
		state.stack.push({ k: 'proc', named: false, opened: true });
		return 'bracket';
	}
	if (stream.match(OPEN_CMD)) {
		state.stack.push({ k: 'cmd', named: false, opened: true });
		return 'bracket';
	}
	if (stream.match(/^<\/?trigger\b/)) {
		state.stack.push({ k: 'trigger' });
		return 'tagName';
	}
	if (stream.match('[[')) {
		state.stack.push({ k: 'doc' });
		return 'bracket';
	}
	if (stream.match('[')) {
		state.stack.push({ k: 'array' });
		return 'bracket';
	}
	if (stream.match('{')) {
		state.stack.push({ k: 'object', key: true });
		return 'bracket';
	}
	if (stream.match('(')) {
		state.stack.push({ k: 'paren' });
		return 'punctuation';
	}
	// Literals
	if (stream.match(RAW_STRING)) return 'string';
	if (stream.match(STRING)) return 'string';
	if (stream.match(NUMBER)) return 'number';
	if (stream.match(/^(?:true|false)\b/)) return 'bool';
	if (stream.match(/^(?:none|null)\b/)) return 'none';
	// Operators (multi-char first)
	if (stream.match(/^(?:==|!=|<=|>=|&&|\|\|)/)) return 'operator';
	if (stream.match(/^[+\-*/<>!]/)) return 'operator';
	// Punctuation. `(` opened a frame above, so only its closer lands here.
	if (stream.match(/^[),]/)) return 'punctuation';
	if (stream.match(/^:/)) return 'punctuation';
	// Sugar for serde's externally tagged form, so it reads as a literal.
	if (stream.match(VARIANT)) return 'variant';
	// A reference: `scope:name.path`, or a bare `name.path` binding.
	if (stream.match(NAMED)) return 'scope';
	if (stream.match(DOTTED)) return 'varName';
	// A field suffix on whatever came before: `items[0].name`, `@[p()].id`.
	if (stream.match(FIELD)) return 'varName';
	return null;
}

/** A module's top level: `export name = value` and `export proc name(params)`. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function moduleTop(stream: any, state: State): string | null {
	if (stream.eatSpace()) return null;
	if (stream.match(COMMENT)) return 'comment';

	switch (state.phase) {
		case 'export':
			if (stream.match(/^proc\b/)) {
				state.phase = 'procName';
				return 'declKeyword';
			}
			state.phase = 'top';
			if (stream.match(IDENT)) return 'def';
			break;
		case 'procName':
			state.phase = 'sig';
			if (stream.match(IDENT)) return 'defFunc';
			break;
		case 'returns':
			if (stream.match('->')) return 'operator';
			if (stream.match(/^none\b/) || stream.match(PARAM_TYPE)) return 'type';
			// Anything else on the signature line is already the body.
			state.phase = 'body';
			return parser.token(stream, state);
		case 'sig':
			if (stream.match('(')) {
				state.stack.push({ k: 'params' });
				return 'punctuation';
			}
			break;
	}

	if (stream.match(/^export\b/)) {
		state.phase = 'export';
		return 'declKeyword';
	}
	if (stream.match(/^=(?!=)/)) return 'operator';
	return exprAtom(stream, state) ?? (stream.next(), null);
}

const parser: StreamParser<State> = {
	startState: () => ({ stack: [] }),

	copyState: (s) => ({ stack: s.stack.map((f) => ({ ...f })), phase: s.phase }),

	token(stream, state) {
		const top = state.stack[state.stack.length - 1];

		if (!top) {
			if (stream.match(OPEN_VAR)) {
				state.stack.push({ k: 'var', opened: true });
				return 'delimiter';
			}
			if (stream.match(OPEN_TAG)) {
				state.stack.push({ k: 'tag', opened: true });
				return 'delimiter';
			}
			if (stream.match(OPEN_PROC)) {
				state.stack.push({ k: 'proc', named: false, opened: true });
				return 'bracket';
			}
			if (stream.match(OPEN_CMD)) {
				state.stack.push({ k: 'cmd', named: false, opened: true });
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

		const sol = stream.sol();

		if (trimmable(top) && top.opened) {
			top.opened = false;
			if (!sol && stream.match(TRIM_L)) return 'trim';
		}

		if (sol && top.k === 'object') top.key = true;

		if (stream.eatSpace()) return null;

		switch (top.k) {
			// {{ expr }} — usually a reference, but any expression is legal
			case 'var': {
				if (stream.match(TRIM_VAR)) return 'trim';
				if (stream.match(CLOSE_VAR)) {
					state.stack.pop();
					return 'delimiter';
				}
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// {# if ... #} / {# foreach x in y #}
			case 'tag': {
				if (stream.match(TRIM_TAG)) return 'trim';
				if (stream.match(CLOSE_TAG)) {
					state.stack.pop();
					return 'delimiter';
				}
				if (stream.match(KEYWORD)) {
					if (state.phase === 'body' && stream.current() === 'endproc') state.phase = 'top';
					return 'keyword';
				}
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// export proc name(base: number, mult: number = 1)
			case 'params': {
				if (stream.match(')')) {
					state.stack.pop();
					state.phase = 'returns';
					return 'punctuation';
				}
				if (stream.match(NAMED)) return 'param';
				if (stream.match(PARAM_TYPE)) return 'type';
				if (stream.match(/^=(?!=)/)) return 'operator';
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// @[ namespace.name(key: value) ]
			case 'proc': {
				if (stream.match(TRIM_CALL)) return 'trim';
				if (stream.match(CLOSE_CALL)) {
					state.stack.pop();
					return 'bracket';
				}
				if (!top.named && stream.match(DOTTED)) {
					top.named = true;
					return 'func';
				}
				if (top.named && !top.args && stream.match('(')) {
					top.args = true;
					return 'punctuation';
				}
				if (stream.match(NAMED)) return 'propName';
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// $[ name(arg, key: value) ]
			case 'cmd': {
				if (stream.match(TRIM_CALL)) return 'trim';
				if (stream.match(CLOSE_CALL)) {
					state.stack.pop();
					return 'bracket';
				}
				if (!top.named && stream.match(IDENT)) {
					top.named = true;
					return 'func';
				}
				if (top.named && !top.args && stream.match('(')) {
					top.args = true;
					return 'punctuation';
				}
				if (stream.match(NAMED)) return 'propName';
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// [ expr, expr ]
			case 'array': {
				if (stream.match(']')) {
					state.stack.pop();
					return 'bracket';
				}
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// { key: expr, "key": expr }
			case 'object': {
				if (stream.match('}')) {
					state.stack.pop();
					return 'bracket';
				}
				if (top.key) {
					if (stream.match(NAMED)) return 'propName';
					if (stream.match(/^"(?:[^"\\]|\\.)*"(?=\s*:)/)) return 'propName';
					if (stream.match(':')) {
						top.key = false;
						return 'punctuation';
					}
				}
				if (stream.match(',')) {
					top.key = true;
					return 'punctuation';
				}
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// ( expr )
			case 'paren': {
				if (stream.match(')')) {
					state.stack.pop();
					return 'punctuation';
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
				// A dynamic id: `[[{{scope:name}}]]`, `[[@[p()]]]`.
				return exprAtom(stream, state) ?? (stream.next(), null);
			}

			// <trigger id="...">
			case 'trigger': {
				if (stream.match('/>') || stream.match('>')) {
					state.stack.pop();
					return 'tagName';
				}
				if (stream.match(/^[A-Za-z_][\w-]*(?=\s*=)/)) return 'attrName';
				if (stream.match('=')) return 'operator';
				if (stream.match(STRING)) return 'string';
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
		commentTokens: { line: '//' },
		indentOnInput: /^\s*[)\]}]$/
	},

	tokenTable: {
		delimiter: t.brace,
		trim: t.modifier,
		bracket: t.squareBracket,
		keyword: t.controlKeyword,
		scope: t.namespace,
		varName: t.variableName,
		func: t.function(t.variableName),
		propName: t.propertyName,
		variant: t.typeName,
		string: t.string,
		number: t.number,
		bool: t.bool,
		none: t.null,
		operator: t.operator,
		punctuation: t.punctuation,
		comment: t.lineComment,
		tagName: t.tagName,
		attrName: t.attributeName,
		docId: t.labelName,
		declKeyword: t.definitionKeyword,
		def: t.definition(t.variableName),
		defFunc: t.definition(t.function(t.variableName)),
		param: t.definition(t.propertyName),
		type: t.typeName
	}
};

/** The template parser, with a module's exports around its proc bodies. */
const moduleParser: StreamParser<State> = {
	...parser,
	startState: () => ({ stack: [], phase: 'top' }),
	token(stream, state) {
		// A return type ends with its line; the body starts on the next.
		if (state.phase === 'returns' && stream.sol()) state.phase = 'body';
		const outside = state.stack.length === 0 && state.phase !== 'body';
		return outside ? moduleTop(stream, state) : parser.token(stream, state);
	}
};

export const weaverLanguage = StreamLanguage.define(parser);
/** A module: exports at the top level, template bodies inside each proc. */
export const weaverModuleLanguage = StreamLanguage.define(moduleParser);

const weaverHighlight = HighlightStyle.define([
	{ tag: t.brace, color: DELIMITER_COLOR, fontWeight: '600' }, // {{ }} {# #}
	{ tag: t.modifier, color: `var(--weaver-trim-color, ${TRIM_COLOR})`, fontWeight: '700' },
	{ tag: t.squareBracket, color: '#c98a3a', fontWeight: '600' }, // @[ ] $[ ] [[ ]] [ ] { }
	{ tag: t.controlKeyword, color: '#c678dd', fontWeight: '600' },
	{ tag: t.namespace, color: '#56b6c2' }, // scope before colon
	{ tag: t.variableName, color: '#e5c07b' },
	{ tag: t.function(t.variableName), color: '#61afef' }, // processor/command name
	{ tag: t.propertyName, color: '#98c379' }, // arg keys, object keys
	{ tag: t.typeName, color: '#d19a66' }, // Variant(…), a literal in disguise
	{ tag: t.string, color: '#98c379' },
	{ tag: t.number, color: '#d19a66' },
	{ tag: t.bool, color: '#d19a66' },
	{ tag: t.null, color: '#d19a66' },
	{ tag: t.operator, color: '#56b6c2' },
	{ tag: t.punctuation, color: '#828a99' },
	{ tag: t.lineComment, color: '#6b7a8d', fontStyle: 'italic' },
	{ tag: t.tagName, color: '#e06c75', fontWeight: '600' }, // <trigger>
	{ tag: t.attributeName, color: '#d19a66' }, // id=
	{ tag: t.labelName, color: '#e5c07b', textDecoration: 'underline' }, // [[doc]]
	{ tag: t.definitionKeyword, color: '#c678dd', fontWeight: '600' }, // export, proc
	{ tag: t.definition(t.variableName), color: '#e5c07b', fontWeight: '600' }, // exported value
	{ tag: t.definition(t.function(t.variableName)), color: '#61afef', fontWeight: '600' }, // proc
	{ tag: t.definition(t.propertyName), color: '#e06c75' } // a proc's parameter
]);

export const weaverSyntax = [weaverLanguage, syntaxHighlighting(weaverHighlight)];
const weaverModuleSyntax = [weaverModuleLanguage, syntaxHighlighting(weaverHighlight)];

/** The highlighting for one dialect. Only modules read differently. */
export function weaverSyntaxFor(dialect: string) {
	return dialect === 'module' ? weaverModuleSyntax : weaverSyntax;
}
