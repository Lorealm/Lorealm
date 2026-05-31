<script lang="ts">
	import WeaverField from "$lib/weaver/editor/WeaverField.svelte";

	// The one place weaver text is entered. Today it is a plain <textarea>/<input>;
	// swapping in CodeMirror later means replacing the markup below while keeping
	// this controlled `value` / `onchange` contract, so no caller has to change.
	let {
		value,
		onchange,
		multiline = false,
		rows = 10,
		placeholder = '',
		ariaLabel
	}: {
		value: string;
		onchange: (next: string) => void;
		multiline?: boolean;
		rows?: number;
		placeholder?: string;
		ariaLabel?: string;
	} = $props();
</script>

{#if multiline}
	<textarea
		class="weaver-input weaver-input--multiline"
		{rows}
		{placeholder}
		aria-label={ariaLabel}
		{value}
		oninput={(e) => onchange(e.currentTarget.value)}
	></textarea>
{:else}
	<WeaverField
		doc={value}
		onChange={onchange}
	/>
{/if}

<style>
	.weaver-input {
		width: 100%;
		font-family: var(--font-family-mono, ui-monospace, monospace);
		font-size: var(--font-size-sm);
		line-height: 1.5;
	}
	.weaver-input--multiline {
		resize: vertical;
		min-height: 6rem;
	}
</style>
