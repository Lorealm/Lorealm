<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState, type ChangeSpec } from '@codemirror/state';
	import { EditorView, drawSelection, keymap, tooltips } from '@codemirror/view';
	import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
	import { weaverSyntax } from './weaver-lang.js';
	import { weaverHover } from './weaver-hover.js';
	import { weaverCompletion } from './weaver-complete.js';
	import { weaverTheme } from './weaver-theme.js';
	import { weaverReferences } from './weaver-reflinks.js';
	import type { ReferenceSource } from './weaver-references.js';
	import { weaverVocabularyStore } from '$lib/stores/weaverVocabulary.svelte.js';

	let {
		doc = '',
		onChange,
		references
	}: {
		doc?: string;
		onChange?: (value: string) => void;
		/** Supplies entry ids for `[[…]]`/`<trigger>` completion and navigation. */
		references?: ReferenceSource;
	} = $props();

	let host: HTMLDivElement;
	let view: EditorView;

	const fieldSizing = EditorView.theme({
		'&': { borderRadius: '6px', border: '1px solid #23262e' },
		'.cm-content': { padding: '7px 10px' },
		'.cm-scroller': { overflowX: 'auto' }
	});

	// Keep the document to one line.
	const flattenNewlines = EditorState.transactionFilter.of((tr) => {
		if (!tr.docChanged || tr.newDoc.lines === 1) return tr;
		const changes: ChangeSpec[] = [];
		tr.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
			changes.push({
				from: fromA,
				to: toA,
				insert: inserted.toString().replace(/\r?\n/g, ' ')
			});
		});
		return [
			{
				changes,
				selection: tr.selection,
				scrollIntoView: tr.scrollIntoView
			}
		];
	});

	onMount(() => {
		void weaverVocabularyStore.load();
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc,
				extensions: [
					weaverSyntax,
					weaverHover,
					weaverCompletion,
					weaverTheme,
					...(references ? [weaverReferences(references)] : []),
					fieldSizing,
					drawSelection(),
					history(),
					tooltips({ position: 'fixed' }),
					// Enter does nothing in a field; our binding wins by being first.
					keymap.of([{ key: 'Enter', run: () => true }, ...defaultKeymap, ...historyKeymap]),
					flattenNewlines,
					...(onChange
						? [
								EditorView.updateListener.of((u) => {
									if (u.docChanged) onChange(u.state.doc.toString());
								})
							]
						: [])
				]
			})
		});
	});

	onDestroy(() => view?.destroy());
</script>

<div bind:this={host}></div>
