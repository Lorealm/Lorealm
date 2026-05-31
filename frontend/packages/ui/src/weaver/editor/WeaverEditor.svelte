<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState } from '@codemirror/state';
	import {
		EditorView,
		lineNumbers,
		highlightActiveLine,
		highlightActiveLineGutter,
		drawSelection,
		keymap
	} from '@codemirror/view';
	import { history, defaultKeymap, historyKeymap } from '@codemirror/commands';
	import { weaverSyntax } from './weaver-lang.js';
	import { weaverHover } from './weaver-hover.js';
	import { weaverTheme } from './weaver-theme.js';

	let { doc = '', onChange }: { doc?: string; onChange?: (value: string) => void } = $props();
	let host: HTMLDivElement;
	let view: EditorView;

	// Multi-line sizing only; colors/fonts come from the shared weaverTheme.
	const editorSizing = EditorView.theme({
		'&': { height: '100%' },
		'.cm-content': { padding: '12px 0' }
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
					keymap.of([...defaultKeymap, ...historyKeymap]),
					EditorView.lineWrapping, // wrap + line numbers coexist
					weaverSyntax,
					weaverHover,
					weaverTheme,
					editorSizing,
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
