<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState } from '@codemirror/state';
	import { EditorView, drawSelection, keymap, tooltips, placeholder } from '@codemirror/view';
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
		references,
		readOnly = false,
		rows = 6,
		hint = '',
		autofocus = false,
		fill = false,
		surface = 'elevated'
	}: {
		doc?: string;
		onChange?: (value: string) => void;
		/** Supplies entry ids for `[[…]]`/`<trigger>` completion and navigation. */
		references?: ReferenceSource;
		readOnly?: boolean;
		/** Visible lines before the editor starts scrolling. */
		rows?: number;
		hint?: string;
		autofocus?: boolean;
		fill?: boolean;
		/**
		 * `elevated` matches the other form controls. Use `sunken` where the
		 * container is already elevated, so the editor doesn't vanish into it.
		 */
		surface?: 'elevated' | 'sunken';
	} = $props();

	let host: HTMLDivElement;
	let view: EditorView;

	const areaTheme = (() =>
		EditorView.theme({
			'&': {
				width: '100%',
				boxSizing: 'border-box',
				backgroundColor: surface === 'sunken' ? 'var(--color-bg-base)' : 'var(--color-bg-elevated)',
				borderRadius: 'var(--radius-md)',
				border:
					surface === 'sunken' ? '1px solid var(--color-border-subtle)' : '1px solid transparent',
				...(fill ? { height: '100%' } : {})
			},
			'&.cm-focused': {
				outline: 'none',
				borderColor: 'var(--color-border-active)'
			},
			// `.cm-content` is the only part that takes clicks, so it has to span the
			// whole visible box. CodeMirror's own `min-height: 100%` can't do that
			// here: it resolves against `.cm-scroller`, whose height is auto.
			'.cm-content': {
				padding: 'var(--spacing-sm)',
				fontFamily: 'var(--font-family-body)',
				lineHeight: '1.5',
				minHeight: fill ? '100%' : `calc(${rows} * 1.5em + 2 * var(--spacing-sm))`
			},
			'.cm-scroller': {
				overflowY: 'auto',
				lineHeight: '1.5',
				...(fill ? { height: '100%' } : { maxHeight: '60vh' })
			},
			'.cm-placeholder': { color: 'var(--color-text-muted)' }
		}))();

	onMount(() => {
		void weaverVocabularyStore.load();
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc,
				extensions: [
					drawSelection(),
					history(),
					// No `indentWithTab`: Tab should leave the field like it does a textarea.
					keymap.of([...defaultKeymap, ...historyKeymap]),
					EditorView.lineWrapping,
					weaverSyntax,
					weaverHover,
					weaverCompletion,
					weaverTheme,
					...(hint ? [placeholder(hint)] : []),
					...(readOnly ? [EditorState.readOnly.of(true), EditorView.editable.of(false)] : []),
					...(references ? [weaverReferences(references)] : []),
					areaTheme,
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
		if (autofocus) view.focus();
	});

	onDestroy(() => view?.destroy());
</script>

<div class="weaverArea" class:fill bind:this={host}></div>

<style>
	.weaverArea {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.weaverArea.fill {
		flex: 1 1 auto;
		min-height: 0;
	}
	.weaverArea.fill > :global(.cm-editor) {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
