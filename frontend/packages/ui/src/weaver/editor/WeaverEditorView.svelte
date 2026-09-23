<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState } from '@codemirror/state';
	import {
		EditorView,
		lineNumbers,
		highlightActiveLine,
		highlightActiveLineGutter,
		drawSelection,
		keymap,
		tooltips
	} from '@codemirror/view';
	import { indentUnit, indentOnInput } from '@codemirror/language';
	import { indentWithTab } from '@codemirror/commands';
	import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
	import { weaverSyntax } from './weaver-lang.js';
	import { weaverHover } from './weaver-hover.js';
	import { weaverCompletion } from './weaver-complete.js';
	import { weaverClose } from './weaver-close.js';
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
		readOnly = false,
		dialect = 'template',
		profile
	}: {
		doc?: string;
		onChange?: (value: string) => void;
		/** Supplies entry ids for `[[…]]`/`<trigger>` completion and navigation. */
		references?: ReferenceSource;
		/** Supplies variables this editor's surroundings declare, for completion and hover. */
		variables?: VariableSource;
		readOnly?: boolean;
		/** Which compiler to check source against. Condition fields hold an `expression`. */
		dialect?: WeaverDialect;
		/** Which host the source will run in. See {@link weaverDiagnostics}. */
		profile?: WeaverProfile;
	} = $props();
	let host: HTMLDivElement;
	let view: EditorView;

	const editorSizing = EditorView.theme({
		'&': { height: '100%' },
		'.cm-content': { padding: '12px 0' }
	});

	const fieldSizing = EditorView.theme({
		'&': {
			width: '100%',
			boxSizing: 'border-box',
			backgroundColor: 'var(--color-bg-base)',
			borderRadius: '6px',
			border: '1px solid #23262e'
		},
		'.cm-content': { padding: '7px 10px' },
		'.cm-scroller': { overflowX: 'auto' }
	});

	onMount(() => {
		void weaverVocabularyStore.load();
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc,
				extensions: [
					lineNumbers(),
					highlightActiveLineGutter(),
					highlightActiveLine(),
					drawSelection(),
					history(),
					keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
					indentUnit.of('  '),
					indentOnInput(),
					EditorView.lineWrapping,
					weaverSyntax,
					weaverHover,
					weaverCompletion,
					weaverClose,
					weaverTheme,
					weaverDiagnostics(dialect, profile),
					...(readOnly ? [EditorState.readOnly.of(true), EditorView.editable.of(false)] : []),
					...(references ? [weaverReferences(references)] : []),
					...(variables ? [variableSource.of(variables)] : []),
					...(profile ? [editorProfile.of(profile)] : []),
					editorSizing,
					fieldSizing,
					tooltips({ position: 'fixed', parent: document.body }),
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

<div class="editor" bind:this={host} style={trimMarkerStyle()}></div>

<style>
	.editor {
		display: flex;
		flex: 1 0;
		min-height: 50vh;
		height: 100%;
		overflow: hidden;
		border-radius: 8px;
		border: 1px solid var(--color-border-subtle);
	}
</style>
