import { Facet, type EditorState } from '@codemirror/state';
import { weaverVocabularyStore } from '#lib/stores/weaverVocabulary.svelte.js';
import type { NamespaceDef, WeaverProfile } from '#lib/types/weaver.js';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * One variable an editor can offer.
 */
export interface VariableRef {
	scope: string;
	name: string;
	signature: string;
	detail: string;
	doc: string;
}

/** Variables only the surrounding editor knows about */
export interface VariableSource {
	variables: () => VariableRef[];
	elements?: (iterable: string) => VariableRef[] | null;
}

export const editorProfile = Facet.define<WeaverProfile, WeaverProfile | null>({
	combine: (values) => values[0] ?? null
});

export function visibleNamespaces(state: EditorState): NamespaceDef[] {
	const profile = state.facet(editorProfile);
	return weaverVocabularyStore.namespaces.filter(
		(ns) => !profile || ns.profiles.length === 0 || ns.profiles.includes(profile)
	);
}

export const variableSource = Facet.define<VariableSource, VariableSource | null>({
	combine: (values) => values[0] ?? null
});

export function variable(
	scope: string,
	name: string,
	detail: string,
	doc: string,
	returns?: string
): VariableRef {
	const reference = scope ? `${scope}:${name}` : name;
	return {
		scope,
		name,
		signature: returns ? `${reference} -> ${returns}` : reference,
		detail,
		doc
	};
}

const VAR_COMMAND =
	/\$\[\s*(?:default_var|del_var|get_var|inc_var|push_var|set_var|var_exists)\s*\(\s*"([A-Za-z_]\w*):([\w.]+)"/g;
const FOREACH_OPEN = /\{#-?\s*foreach\s+([A-Za-z_]\w*)\s+in\s+([\s\S]*?)-?#\}/g;
const FOREACH_CLOSE = /\{#-?\s*endforeach\s*-?#\}/g;

export interface LoopBinding {
	name: string;
	iterable: string;
}

/** The `foreach` bindings open at `pos`, outermost first. */
export function bindingsAt(text: string, pos: number): LoopBinding[] {
	const upto = text.slice(0, pos);
	const events: Array<{ at: number; opens?: LoopBinding }> = [];
	for (const m of upto.matchAll(FOREACH_OPEN)) {
		events.push({ at: m.index, opens: { name: m[1], iterable: m[2].trim() } });
	}
	for (const m of upto.matchAll(FOREACH_CLOSE)) {
		events.push({ at: m.index });
	}
	events.sort((a, b) => a.at - b.at);

	const open: LoopBinding[] = [];
	for (const event of events) {
		if (event.opens) open.push(event.opens);
		else open.pop();
	}
	return open;
}

/** Variables this body declares about itself. */
function scanDeclared(text: string): VariableRef[] {
	const found: VariableRef[] = [];

	VAR_COMMAND.lastIndex = 0;
	let m: RegExpExecArray | null;
	while ((m = VAR_COMMAND.exec(text))) {
		found.push(variable(m[1], m[2], t.weaver.kinds.declaredHere, t.weaver.writtenByCommand));
	}

	return found;
}

const PROC_OPEN = /\bexport\s+proc\s+([A-Za-z_]\w*)\s*\(/g;
const PROC_CLOSE = /\{#-?\s*endproc\s*-?#\}/;
const PARAM = /^([A-Za-z_]\w*)\s*:\s*([^=]*?)\s*(?:=\s*([\s\S]*?))?\s*$/;

function splitParams(list: string): string[] {
	const out: string[] = [];
	let depth = 0;
	let quoted = false;
	let start = 0;
	for (let i = 0; i < list.length; i++) {
		const c = list[i];
		if (quoted) {
			if (c === '\\') i++;
			else if (c === '"') quoted = false;
		} else if (c === '"') quoted = true;
		else if ('([{<'.includes(c)) depth++;
		else if (')]}>'.includes(c)) depth--;
		else if (c === ',' && depth === 0) {
			out.push(list.slice(start, i));
			start = i + 1;
		}
	}
	out.push(list.slice(start));
	return out;
}

function procParams(text: string, pos: number): VariableRef[] {
	const upto = text.slice(0, pos);
	const open = [...upto.matchAll(PROC_OPEN)].at(-1);
	if (!open) return [];

	let i = open.index + open[0].length;
	let depth = 1;
	while (i < text.length && depth > 0) {
		if (text[i] === '(') depth++;
		else if (text[i] === ')') depth--;
		i++;
	}
	if (depth > 0 || i > pos || PROC_CLOSE.test(upto.slice(i))) return [];

	const proc = open[1];
	const list = text.slice(open.index + open[0].length, i - 1).replace(/\/\/.*$/gm, '');
	return splitParams(list).flatMap((param) => {
		const m = PARAM.exec(param.trim());
		if (!m) return [];
		const [, name, type, fallback] = m;
		const doc = fallback
			? t.weaver.parameterDefault({ proc, fallback })
			: t.weaver.parameterOf({ proc });
		return [variable('input', name, t.weaver.kinds.parameter, doc, type)];
	});
}

function loopRefs(state: EditorState, pos: number): VariableRef[] {
	const source = state.facet(variableSource);
	return bindingsAt(state.doc.toString(), pos).flatMap((loop) => [
		variable(
			'',
			loop.name,
			t.weaver.kinds.loopBinding,
			t.weaver.eachElement({ iterable: loop.iterable })
		),
		...(source?.elements?.(loop.iterable) ?? []).map((field) => ({
			...field,
			scope: loop.name,
			signature: `${loop.name}.${field.signature}`
		}))
	]);
}

export function knownVariables(state: EditorState, pos: number): VariableRef[] {
	const builtin = visibleNamespaces(state).flatMap((ns) =>
		ns.members.map((m) => variable(ns.scope, m.name, 'built-in', m.description, m.returns))
	);
	const supplied = state.facet(variableSource)?.variables() ?? [];

	const seen = new Set<string>();
	return [
		...builtin,
		...supplied,
		...loopRefs(state, pos),
		...procParams(state.doc.toString(), pos),
		...scanDeclared(state.doc.toString())
	].filter((ref) => {
		const key = `${ref.scope}:${ref.name}`;
		if (seen.has(key)) return false;
		seen.add(key);
		return true;
	});
}

export function findVariable(
	refs: VariableRef[],
	scope: string,
	name: string
): VariableRef | undefined {
	const exact = refs.find((r) => r.scope === scope && r.name === name);
	if (exact) return exact;
	// `{{c.handle}}` reads as one bare word; the binding is its first segment.
	if (scope === '' && name.includes('.')) {
		const cut = name.indexOf('.');
		return findVariable(refs, name.slice(0, cut), name.slice(cut + 1));
	}
	const pattern = refs.find((r) => {
		if (r.scope !== scope || !r.name.includes('<')) return false;
		const pattern = r.name
			.split(/<\w+>/)
			.map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
			.join('\\w+');
		return new RegExp(`^${pattern}$`).test(name);
	});
	if (pattern) return pattern;
	// `{{input:lead.id}}` reads a field of the input `lead`.
	if (scope === 'input' && name.includes('.')) {
		return findVariable(refs, scope, name.slice(0, name.indexOf('.')));
	}
	return undefined;
}
