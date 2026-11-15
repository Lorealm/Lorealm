import { hoverTooltip, type Tooltip } from '@codemirror/view';
import {
	formatCommandSignature,
	formatProcessorSignature,
	paramDocs,
	weaverVocabularyStore
} from '#lib/stores/weaverVocabulary.svelte.js';
import { keywords } from './weaver-meta.js';
import { activeConstruct, type Construct } from './weaver-context.js';
import { infoDom, type ParamDoc } from './weaver-render.js';
import { findVariable, knownVariables, type VariableRef } from './weaver-variables.js';
import { messageStateStore } from '#lib/stores/messageState.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

/** The scope of a reference, when the hovered word is its name. */
const SCOPE_BEFORE = /([A-Za-z_]\w*)\s*:\s*$/;
/** The name of a reference, when the hovered word is its scope. */
const NAME_AFTER = /^\s*:\s*([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)/;

/**
 * The `scope:name` (or bare `name`) reference the hovered word belongs to.
 */
function referenceEntry(
	text: string,
	wordStart: number,
	word: string,
	ctx: Construct,
	refs: VariableRef[]
): { signature: string; doc: string } | null {
	const inCall = ctx === 'proc' || ctx === 'cmd';
	const scoped = NAME_AFTER.exec(text.slice(wordStart + word.length));
	const before = SCOPE_BEFORE.exec(text.slice(0, wordStart));

	let scope: string | undefined;
	let name: string;
	if (before) {
		scope = before[1];
		name = word;
	} else if (scoped && !inCall) {
		scope = word;
		name = scoped[1];
	} else if (inCall) {
		return null;
	} else {
		name = word;
	}

	if (!scope && ctx !== 'var') return null;

	const reference = scope ? `${scope}:${name}` : name;
	const known = findVariable(refs, scope ?? '', name);
	const preview = scope ? messageStateStore.previewVariable(reference) : undefined;
	const live =
		preview !== undefined ? t.weaver.resolvesTo({ value: String(preview) }) : t.weaver.noValue;
	return {
		signature: known?.signature ?? (ctx === 'var' ? `{{${reference}}}` : reference),
		doc: known ? `${known.doc}\n\n${live}` : live
	};
}

export const weaverHover = hoverTooltip((view, pos): Tooltip | null => {
	const line = view.state.doc.lineAt(pos);
	const text = line.text;
	const col = pos - line.from;

	const wordRe = /[A-Za-z_][\w.]*/g;
	let word = '';
	let wordStart = -1;
	let m: RegExpExecArray | null;
	while ((m = wordRe.exec(text))) {
		const s = m.index;
		const e = s + m[0].length;
		if (col >= s && col <= e) {
			word = m[0];
			wordStart = s;
			break;
		}
		if (s > col) break;
	}
	if (!word) return null;

	const before = text.slice(0, wordStart);
	const ctx = activeConstruct(before);
	const from = line.from + wordStart;
	const to = from + word.length;

	let entry: { signature: string; doc: string; params?: ParamDoc[] } | null = null;

	if (ctx === 'proc') {
		const proc = weaverVocabularyStore.processor(word);
		if (proc)
			entry = {
				signature: formatProcessorSignature(proc),
				doc: proc.description,
				params: paramDocs(proc)
			};
	} else if (ctx === 'cmd') {
		const cmd = weaverVocabularyStore.command(word);
		if (cmd)
			entry = {
				signature: formatCommandSignature(cmd),
				doc: cmd.description,
				params: paramDocs(cmd)
			};
	} else if (ctx === 'tag' && keywords[word]) {
		const e = keywords[word];
		entry = { signature: e.signature, doc: e.doc };
	}

	if (!entry && ctx && ctx !== 'doc') {
		entry = referenceEntry(text, wordStart, word, ctx, knownVariables(view.state, pos));
	}

	if (!entry) return null;

	return {
		pos: from,
		end: to,
		above: true,
		create: () => ({ dom: infoDom(entry!.signature, entry!.doc, entry!.params) })
	};
});
