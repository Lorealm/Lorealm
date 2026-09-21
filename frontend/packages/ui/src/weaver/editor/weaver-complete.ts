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
import type { NamespaceDef } from '$lib/types/weaver.js';
import { keywords } from './weaver-meta.js';
import { activeConstruct } from './weaver-context.js';
import { infoDom, type ParamDoc } from './weaver-render.js';
import { idSiteAt, referenceSource, writableAt, type IdSite } from './weaver-references.js';
import {
	bindingsAt,
	knownVariables,
	visibleNamespaces,
	type VariableRef
} from './weaver-variables.js';

const TOKEN = /[\w.]*/;
const SCOPED = /([A-Za-z_]\w*)\s*:\s*([\w.]*)$/;
const BOUND = /([A-Za-z_]\w*)\.([\w.]*)$/;
const PLACEHOLDER = /^<.*>$/;
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

function variableOption(ref: VariableRef): Completion {
	return option(ref.name, ref.detail, ref.signature, ref.doc);
}

function scopeOptions(refs: VariableRef[], namespaces: NamespaceDef[]): Completion[] {
	const documented = new Map(namespaces.map((ns) => [ns.scope, ns]));
	const scopes = new Set([
		...documented.keys(),
		...refs.map((r) => r.scope).filter((scope) => scope !== '')
	]);

	return [
		...[...scopes].map((scope) => ({
			label: scope,
			detail: 'namespace',
			apply: `${scope}:`,
			info: () =>
				infoDom(
					`${scope}:`,
					documented.get(scope)?.description ?? 'A state namespace this document uses.'
				)
		})),
		...refs.filter((r) => r.scope === '').map(variableOption)
	];
}

/** What sits directly under a path, for a group's info panel. */
function holds(group: VariableRef[], prefix: string): string {
	const heads = [...new Set(group.map((r) => r.name.slice(prefix.length).split('.')[0]))];
	const shown = heads
		.slice(0, 6)
		.map((head) => `\`${head}\``)
		.join(', ');
	return `Holds ${shown}${heads.length > 6 ? ', and more' : ''}.`;
}

function memberCompletions(
	context: CompletionContext,
	scope: string,
	path: string,
	sep: ':' | '.' = ':'
): CompletionResult | null {
	const dot = path.lastIndexOf('.');
	const prefix = dot < 0 ? '' : path.slice(0, dot + 1);
	const typed = path.slice(dot + 1);

	const under = knownVariables(context.state, context.pos).filter(
		(r) => r.scope === scope && r.name !== '' && r.name.startsWith(prefix)
	);
	if (under.length === 0) return null;

	const leaves = new Map<string, VariableRef>();
	const groups = new Map<string, VariableRef[]>();
	for (const ref of under) {
		const rest = ref.name.slice(prefix.length);
		const cut = rest.indexOf('.');
		if (cut < 0) {
			leaves.set(rest, ref);
			continue;
		}
		const head = rest.slice(0, cut);
		groups.set(head, [...(groups.get(head) ?? []), ref]);
	}

	const named = [...groups.keys()].filter((head) => !PLACEHOLDER.test(head));
	if (named.length > 0) {
		for (const head of groups.keys()) if (PLACEHOLDER.test(head)) groups.delete(head);
	}

	const options: Completion[] = [
		...[...leaves].map(([label, ref]) => option(label, ref.detail, ref.signature, ref.doc)),
		...[...groups]
			.filter(([label]) => !leaves.has(label))
			.map(([label, group]) => ({
				label,
				detail: 'group',
				apply: `${label}.`,
				info: () => infoDom(`${scope}${sep}${prefix}${label}.…`, holds(group, `${prefix}${label}.`))
			}))
	];

	return {
		from: context.pos - typed.length,
		options,
		validFor: /^\w*$/
	};
}

function inArguments(before: string): boolean {
	const open = Math.max(before.lastIndexOf('@['), before.lastIndexOf('$['));
	return open >= 0 && before.includes('(', open);
}

function optionsFor(
	ctx: Exclude<ReturnType<typeof activeConstruct>, null>,
	context: CompletionContext,
	before: string
): Completion[] {
	const variables = () =>
		scopeOptions(knownVariables(context.state, context.pos), visibleNamespaces(context.state));
	switch (ctx) {
		case 'proc':
			return [
				...weaverVocabularyStore.processors.map((p) =>
					option(
						processorKey(p),
						'processor',
						formatProcessorSignature(p),
						p.description,
						paramDocs(p)
					)
				),
				...(inArguments(before) ? variables() : [])
			];
		case 'cmd':
			return [
				...weaverVocabularyStore.commands.map((c) =>
					option(c.name, 'command', formatCommandSignature(c), c.description, paramDocs(c))
				),
				...(inArguments(before) ? variables() : [])
			];
		case 'tag':
			return [
				...Object.entries(keywords).map(([name, e]) => option(name, 'keyword', e.signature, e.doc)),
				...variables()
			];
		case 'var':
			return variables();
		default:
			return [];
	}
}

function idCompletions(context: CompletionContext, site: IdSite): CompletionResult | null {
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

	const scoped = SCOPED.exec(before);
	const members = scoped && memberCompletions(context, scoped[1], scoped[2]);
	if (members) return members;

	const bound = !scoped && BOUND.exec(before);
	if (
		bound &&
		bindingsAt(context.state.doc.toString(), context.pos).some((b) => b.name === bound[1])
	) {
		const fields = memberCompletions(context, bound[1], bound[2], '.');
		if (fields) return fields;
	}

	const options = optionsFor(ctx, context, before);
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
