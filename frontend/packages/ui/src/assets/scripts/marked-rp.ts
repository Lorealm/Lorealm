import { Marked } from "marked";

export const marked = new Marked();

	// straight " , left curly " , right curly " — any as either delimiter
	const QUOTE = '["\u201C\u201D]';
	const QUOTED_RULE = new RegExp(`^${QUOTE}([^"\u201C\u201D]+)${QUOTE}`);

	marked.use({
		extensions: [
			{
				name: 'quoted',
				level: 'inline',
				start(src) {
					return src.search(/["\u201C\u201D]/);
				},
				tokenizer(src) {
					const match = QUOTED_RULE.exec(src);
					if (!match) return undefined;
					const text = match[1];
					return {
						type: 'quoted',
						raw: match[0],
						text,
						tokens: this.lexer.inlineTokens(text) // inferred Token[], not never[]
					};
				},
				renderer(token) {
					const open = token.raw[0];
					const close = token.raw[token.raw.length - 1];
					return `<span class="quoted">${open}${this.parser.parseInline(token.tokens ?? [])}${close}</span>`;
				}
			}
		]
	});