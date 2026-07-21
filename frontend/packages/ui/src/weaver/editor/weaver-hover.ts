import { hoverTooltip, type Tooltip } from '@codemirror/view';
import {
	formatCommandSignature,
	formatProcessorSignature,
	paramDocs,
	weaverVocabularyStore
} from '$lib/stores/weaverVocabulary.svelte.js';
import { keywords } from './weaver-meta.js';
import { activeConstruct } from './weaver-context.js';
import { infoDom, type ParamDoc } from './weaver-render.js';
import { messageStateStore } from '$lib/stores/messageState.svelte.js';

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
	} else if (ctx === 'var') {
		// Resolve the enclosing {{ scope:name }} for a preview.
		const open = before.lastIndexOf('{{');
		const close = text.indexOf('}}', open);
		const inner = text.slice(open + 2, close === -1 ? text.length : close).trim();
		const [scope, name] = inner.split(':').map((s) => s.trim());
		const sig = name ? `{{${scope}:${name}}}` : `{{${scope}}}`;
		const preview = name ? messageStateStore.previewVariable(`${scope}:${name}`) : undefined;
		const doc =
			preview !== undefined
				? `Resolves to \`${preview}\` for the loaded context.`
				: 'No loaded value for this variable in the current context.';
		entry = { signature: sig, doc };
	}

	if (!entry) return null;

	return {
		pos: from,
		end: to,
		above: true,
		create: () => ({ dom: infoDom(entry!.signature, entry!.doc, entry!.params) })
	};
});
