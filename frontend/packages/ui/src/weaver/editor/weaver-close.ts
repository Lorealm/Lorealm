import { EditorView } from '@codemirror/view';

const PAIRS: { open: string; lead: string; close: string }[] = [
	{ open: '{{', lead: '', close: '}}' },
	{ open: '[[', lead: '', close: ']]' },
	{ open: '@[', lead: '', close: ']' },
	{ open: '$[', lead: '', close: ']' },
	{ open: '{#', lead: ' ', close: ' #}' }
];

const SKIP = new Set(['}', ']', ')']);

export const weaverClose = EditorView.inputHandler.of((view, from, to, text) => {
	if (text.length !== 1 || from !== to || view.state.readOnly) return false;
	if (view.state.selection.ranges.length !== 1) return false;

	if (SKIP.has(text) && view.state.sliceDoc(from, from + 1) === text) {
		view.dispatch({ selection: { anchor: from + 1 }, userEvent: 'move.drop' });
		return true;
	}

	if (from === 0) return false;
	const pair = PAIRS.find((p) => p.open === view.state.sliceDoc(from - 1, from) + text);
	if (!pair) return false;

	view.dispatch({
		changes: { from, insert: text + pair.lead + pair.close },
		selection: { anchor: from + text.length + pair.lead.length },
		userEvent: 'input.type',
		scrollIntoView: true
	});
	return true;
});
