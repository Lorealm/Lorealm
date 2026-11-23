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
	import {
		HighlightStyle,
		bracketMatching,
		indentOnInput,
		indentUnit,
		syntaxHighlighting
	} from '@codemirror/language';
	import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
	import {
		autocompletion,
		closeBrackets,
		closeBracketsKeymap,
		completionKeymap,
		type CompletionContext,
		type CompletionResult
	} from '@codemirror/autocomplete';
	import { html } from '@codemirror/lang-html';
	import { tags as t } from '@lezer/highlight';
	import { weaverTheme } from '#lib/weaver/editor/weaver-theme.js';
	import { rootVariableEntries } from '#lib/core/frameStyle.js';

	let {
		doc = '',
		onChange,
		readOnly = false
	}: {
		doc?: string;
		onChange?: (value: string) => void;
		readOnly?: boolean;
	} = $props();

	let host: HTMLDivElement;
	let view: EditorView;

	/** Same palette as the weaver editor. */
	const highlight = HighlightStyle.define([
		{ tag: [t.tagName, t.angleBracket], color: '#e06c75' },
		{ tag: t.attributeName, color: '#d19a66' },
		{ tag: [t.string, t.attributeValue], color: '#98c379' },
		{ tag: [t.number, t.bool, t.null, t.unit], color: '#d19a66' },
		{ tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.modifier], color: '#c678dd' },
		{ tag: [t.function(t.variableName), t.function(t.propertyName)], color: '#61afef' },
		{ tag: [t.propertyName, t.className], color: '#56b6c2' },
		{ tag: [t.variableName, t.special(t.variableName)], color: '#e5c07b' },
		{ tag: [t.operator, t.punctuation], color: '#828a99' },
		{ tag: t.comment, color: '#6b7a8d', fontStyle: 'italic' },
		{ tag: t.invalid, color: 'var(--color-error-text)' }
	]);

	/** The app's style variables, wherever `--` is typed outside an HTML comment opener. */
	function styleVariables(context: CompletionContext): CompletionResult | null {
		const word = context.matchBefore(/(?<![\w!-])--[\w-]*/);
		if (!word) return null;
		return {
			from: word.from,
			options: rootVariableEntries().map(([name, value]) => ({
				label: name,
				detail: value,
				type: 'variable'
			})),
			validFor: /^--[\w-]*$/
		};
	}

	const sizing = EditorView.theme({
		'&': { height: '100%' },
		'&.cm-focused': { outline: 'none' },
		'.cm-content': { padding: '8px 0' },
		'.cm-scroller': { overflow: 'auto' }
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
					bracketMatching(),
					closeBrackets(),
					autocompletion(),
					keymap.of([
						...closeBracketsKeymap,
						...completionKeymap,
						...defaultKeymap,
						...historyKeymap,
						indentWithTab
					]),
					indentUnit.of('  '),
					indentOnInput(),
					html(),
					EditorState.languageData.of(() => [{ autocomplete: styleVariables }]),
					syntaxHighlighting(highlight),
					weaverTheme,
					sizing,
					tooltips({ position: 'fixed', parent: document.body }),
					...(readOnly ? [EditorState.readOnly.of(true)] : []),
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
		flex: 1;
		min-height: 0;
		overflow: hidden;
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border-subtle);
		background: var(--color-bg-base);
	}
	.editor:focus-within {
		border-color: var(--color-border-active);
	}
</style>
