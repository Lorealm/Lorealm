<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState, type ChangeSpec } from '@codemirror/state';
	import { EditorView, drawSelection, keymap, tooltips, placeholder } from '@codemirror/view';
	import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
	import { weaverSyntax } from './weaver-lang.js';
	import { weaverHover } from './weaver-hover.js';
	import { weaverCompletion } from './weaver-complete.js';
	import { trimMarkerStyle, weaverTheme } from './weaver-theme.js';
	import { weaverDiagnostics } from './weaver-diagnostics.js';
	import { weaverReferences } from './weaver-reflinks.js';
	import type { ReferenceSource } from './weaver-references.js';
	import { editorProfile, variableSource, type VariableSource } from './weaver-variables.js';
	import type { WeaverDialect, WeaverProfile } from '$lib/types/weaver.js';
	import { weaverVocabularyStore } from '$lib/stores/weaverVocabulary.svelte.js';

	let {
		doc = '',
		onChange,
		references,
		variables,
		hint = '',
		dialect = 'template',
		profile
	}: {
		doc?: string;
		onChange?: (value: string) => void;
		/** Supplies entry ids for `[[…]]`/`<trigger>` completion and navigation. */
		references?: ReferenceSource;
		variables?: VariableSource;
		/** Placeholder shown while the field is empty. */
		hint?: string;
		/** Which compiler to check source against. Condition fields hold an `expression`. */
		dialect?: WeaverDialect;
		/** Which host the source will run in. See {@link weaverDiagnostics}. */
		profile?: WeaverProfile;
	} = $props();

	let host: HTMLDivElement;
	let view: EditorView;

	const fieldSizing = EditorView.theme({
		'&': {
			backgroundColor: 'var(--color-bg-base)',
			borderRadius: '6px',
			border: '1px solid #23262e'
		},
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
					weaverDiagnostics(dialect, profile),
					...(hint ? [placeholder(hint)] : []),
					...(references ? [weaverReferences(references)] : []),
					...(variables ? [variableSource.of(variables)] : []),
					...(profile ? [editorProfile.of(profile)] : []),
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

<div bind:this={host} style={trimMarkerStyle()}></div>
