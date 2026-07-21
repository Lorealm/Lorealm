import {
	autocompletion,
	completionKeymap,
	type Completion,
	type CompletionContext,
	type CompletionResult
} from '@codemirror/autocomplete';
import { keymap } from '@codemirror/view';
import { Prec } from '@codemirror/state';
import {
	formatCommandSignature,
	formatProcessorSignature,
	paramDocs,
	processorKey,
	weaverVocabularyStore
} from '$lib/stores/weaverVocabulary.svelte.js';
import { keywords } from './weaver-meta.js';
import { activeConstruct } from './weaver-context.js';
import { infoDom, type ParamDoc } from './weaver-render.js';
import { idSiteAt, referenceSource, writableAt, type IdSite } from './weaver-references.js';

const TOKEN = /[\w.]*/;
const ID_TOKEN: Record<IdSite, RegExp> = { doc: /\w*/, trigger: /[^"]*/ };
const ID_VALID_FOR: Record<IdSite, RegExp> = { doc: /^\w*$/, trigger: /^[^"]*$/ };

function option(
	label: string,
	detail: string,
	signature: string,
	doc: string,
	params: ParamDoc[] = []
): Completion {
	return {
		label,
		detail,
		info: () => infoDom(signature, doc, params)
	};
}

function optionsFor(ctx: Exclude<ReturnType<typeof activeConstruct>, null>): Completion[] {
	switch (ctx) {
		case 'proc':
			return weaverVocabularyStore.processors.map((p) =>
				option(
					processorKey(p),
					'processor',
					formatProcessorSignature(p),
					p.description,
					paramDocs(p)
				)
			);
		case 'cmd':
			return weaverVocabularyStore.commands.map((c) =>
				option(c.name, 'command', formatCommandSignature(c), c.description, paramDocs(c))
			);
		case 'tag':
			return Object.entries(keywords).map(([name, e]) =>
				option(name, 'keyword', e.signature, e.doc)
			);
		default:
			// `var` has no vocabulary to complete against; `doc` is an id site
			// and was already handled before this dispatch.
			return [];
	}
}

function idCompletions(
	context: CompletionContext,
	site: IdSite
): CompletionResult | null {
	const refs = context.state.facet(referenceSource)?.entries() ?? [];
	const options = refs
		.filter((ref) => writableAt(site, ref.id))
		.map((ref) => {
			const doc = [ref.name, ref.detail && `In ${ref.detail}.`].filter(Boolean).join('\n\n');
			return option(ref.id, ref.detail ?? 'entry', ref.id, doc);
		});
	if (options.length === 0) return null;

	const word = context.matchBefore(ID_TOKEN[site]);
	if ((!word || word.from === word.to) && !context.explicit) return null;

	return {
		from: word ? word.from : context.pos,
		options,
		validFor: ID_VALID_FOR[site]
	};
}

function source(context: CompletionContext): CompletionResult | null {
	const line = context.state.doc.lineAt(context.pos);
	const before = line.text.slice(0, context.pos - line.from);

	const site = idSiteAt(before);
	if (site) return idCompletions(context, site);

	const ctx = activeConstruct(before);
	if (!ctx) return null;

	const options = optionsFor(ctx);
	if (options.length === 0) return null;

	const word = context.matchBefore(TOKEN);
	// Don't pop up on an empty token unless the user asked for it explicitly.
	if ((!word || word.from === word.to) && !context.explicit) return null;

	return {
		from: word ? word.from : context.pos,
		options,
		validFor: /^[\w.]*$/
	};
}

export const weaverCompletion = [
	autocompletion({ override: [source], defaultKeymap: false }),
	Prec.highest(keymap.of(completionKeymap))
];
