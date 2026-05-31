import { hoverTooltip, type Tooltip } from '@codemirror/view';
import { commands, keywords, processors, variablePreviews } from './weaver-meta.js';

// Which construct, if any, encloses `pos` on its line. Operates on the line
// text before the hovered word. Good enough for a testbed: weaver-lang body
// constructs are effectively single-line, so we don't track multi-line nesting.
type Construct = 'var' | 'proc' | 'cmd' | 'tag' | 'doc' | null;

const OPENERS: { k: Exclude<Construct, null>; open: string; close: string }[] = [
	{ k: 'var', open: '{{', close: '}}' },
	{ k: 'tag', open: '{#', close: '#}' },
	{ k: 'proc', open: '@[', close: ']' },
	{ k: 'cmd', open: '$[', close: ']' },
	{ k: 'doc', open: '[[', close: ']]' }
];

function activeConstruct(before: string): Construct {
	let best: Construct = null;
	let bestIdx = -1;
	for (const o of OPENERS) {
		const oi = before.lastIndexOf(o.open);
		if (oi === -1) continue;
		const closed = before.indexOf(o.close, oi + o.open.length) !== -1;
		if (!closed && oi > bestIdx) {
			bestIdx = oi;
			best = o.k;
		}
	}
	return best;
}

// Tiny markdown: escape, then `code` -> <code>, blank lines -> paragraphs.
function renderDoc(md: string): string {
	const esc = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	const withCode = esc.replace(/`([^`]+)`/g, '<code>$1</code>');
	return withCode
		.split(/\n\n+/)
		.map((p) => `<p>${p.replace(/\n/g, '<br>')}</p>`)
		.join('');
}

function tooltipDom(title: string, signature: string, doc: string): HTMLElement {
	const root = document.createElement('div');
	root.className = 'cm-weaver-tooltip';
	const sig = document.createElement('div');
	sig.className = 'cm-weaver-tooltip-sig';
	sig.textContent = signature || title;
	const body = document.createElement('div');
	body.className = 'cm-weaver-tooltip-doc';
	body.innerHTML = renderDoc(doc);
	root.append(sig, body);
	return root;
}

export const weaverHover = hoverTooltip((view, pos): Tooltip | null => {
	const line = view.state.doc.lineAt(pos);
	const text = line.text;
	const col = pos - line.from;

	// Word under cursor (allow dots for dotted processor names).
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

	let entry: { title: string; signature: string; doc: string } | null = null;

	if (ctx === 'proc' && processors[word]) {
		const e = processors[word];
		entry = { title: word, signature: e.signature, doc: e.doc };
	} else if (ctx === 'cmd' && commands[word]) {
		const e = commands[word];
		entry = { title: word, signature: e.signature, doc: e.doc };
	} else if (ctx === 'tag' && keywords[word]) {
		const e = keywords[word];
		entry = { title: word, signature: e.signature, doc: e.doc };
	} else if (ctx === 'var') {
		// Resolve the enclosing {{ scope:name }} for a preview.
		const open = before.lastIndexOf('{{');
		const close = text.indexOf('}}', open);
		const inner = text.slice(open + 2, close === -1 ? text.length : close).trim();
		const [scope, name] = inner.split(':').map((s) => s.trim());
		const sig = name ? `{{${scope}:${name}}}` : `{{${scope}}}`;
		const preview = name ? variablePreviews[scope]?.[name] : undefined;
		const doc =
			preview !== undefined
				? `Resolves to \`${preview}\` for the loaded context.`
				: 'No loaded value for this variable in the current context.';
		entry = { title: sig, signature: sig, doc };
	}

	if (!entry) return null;

	return {
		pos: from,
		end: to,
		above: true,
		create: () => ({ dom: tooltipDom(entry!.title, entry!.signature, entry!.doc) })
	};
});
