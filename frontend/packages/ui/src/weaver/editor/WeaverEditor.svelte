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
	import { weaverTheme } from './weaver-theme.js';

	let { doc = '', onChange }: { doc?: string; onChange?: (value: string) => void } = $props();
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
			borderRadius: '6px',
			border: '1px solid #23262e'
		},
		'.cm-content': { padding: '7px 10px' },
		'.cm-scroller': { overflowX: 'auto' }
	});

	onMount(() => {
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
					weaverTheme,
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

<div class="editor" bind:this={host}></div>

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
