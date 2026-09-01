<script lang="ts">
	import WeaverArea from '$lib/weaver/editor/WeaverArea.svelte';
	import WeaverField from '$lib/weaver/editor/WeaverField.svelte';
	import type { WeaverDialect } from '$lib/types/weaver.js';
	import type { ReferenceSource } from '$lib/weaver/editor/weaver-references.js';
	import FieldShell from './FieldShell.svelte';

	let {
		label,
		value,
		onchange,
		hint,
		placeholder = '',
		multiline = true,
		rows = 6,
		references,
		readOnly = false,
		tokens = true,
		maximizable = true,
		weaver = true,
		surface = 'elevated',
		dialect = 'template'
	}: {
		label?: string;
		value: string;
		onchange: (next: string) => void;
		/** Explanatory text behind a `?` next to the label. */
		hint?: string;
		placeholder?: string;
		multiline?: boolean;
		rows?: number;
		/** Supplies entry ids for `[[…]]`/`<trigger>` completion and navigation. */
		references?: ReferenceSource;
		readOnly?: boolean;
		/** Show the running token cost. */
		tokens?: boolean;
		maximizable?: boolean;
		/**
		 * Highlight and complete weaver syntax.
		 */
		weaver?: boolean;
		/** See {@link WeaverArea}'s `surface`. Multiline weaver fields only. */
		surface?: 'elevated' | 'sunken';
		/** See {@link WeaverArea}'s `dialect`. Weaver fields only. */
		dialect?: WeaverDialect;
	} = $props();
</script>

{#snippet editor(fill: boolean)}
	{#if !weaver}
		<textarea
			class="plain"
			class:fill
			rows={multiline ? rows : 1}
			{placeholder}
			readonly={readOnly}
			{value}
			oninput={(e) => onchange(e.currentTarget.value)}
		></textarea>
	{:else if multiline}
		<WeaverArea
			doc={value}
			onChange={readOnly ? undefined : onchange}
			{references}
			{readOnly}
			{rows}
			{fill}
			{surface}
			{dialect}
			hint={placeholder}
			autofocus={fill}
		/>
	{:else}
		<WeaverField doc={value} onChange={readOnly ? undefined : onchange} {references} {dialect} />
	{/if}
{/snippet}

<FieldShell {label} {hint} {maximizable} tokenText={tokens ? value : undefined} {editor} />

<style>
	.plain {
		resize: vertical;
		line-height: 1.5;
	}
	.plain.fill {
		flex: 1 1 auto;
		min-height: 0;
		resize: none;
	}
</style>
