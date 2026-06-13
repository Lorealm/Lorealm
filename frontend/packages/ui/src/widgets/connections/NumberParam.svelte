<script lang="ts">
	import type { NumberSpec } from '$lib/types/schema.js';
	import NumberField from '$lib/components/NumberInput.svelte';

	let {
		label,
		spec,
		value,
		onChange
	}: {
		label: string;
		spec: NumberSpec;
		/** undefined = unset, provider default applies */
		value: number | undefined;
		onChange: (v: number | undefined) => void;
	} = $props();

	const shown = $derived(value ?? spec.default);
	const overridden = $derived(value !== undefined);

	// Slider only — range inputs are never empty, so this always yields a number.
	function slide(e: Event) {
		const n = Number((e.currentTarget as HTMLInputElement).value);
		if (!Number.isNaN(n)) onChange(n);
	}
</script>

<div class="param" class:overridden>
	<div class="head">
		<span class="label">{label}</span>
		{#if overridden}
			<button
				class="reset"
				type="button"
				title="Use provider default"
				onclick={() => onChange(undefined)}>↺</button
			>
		{/if}
		<NumberField
			value={shown}
			onchange={(v) => onChange(v === '' ? undefined : v)}
			min={spec.min}
			max={spec.max}
			step={spec.step}
			width="5.5em"
			ariaLabel={label}
		/>
	</div>
	<input
		class="slider"
		type="range"
		min={spec.min}
		max={spec.max}
		step={spec.step}
		value={shown}
		oninput={slide}
		aria-label={label}
	/>
</div>

<style>
	.param {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.label {
		flex: 1 1 auto;
		font-size: var(--font-size-sm);
	}
	.param:not(.overridden) .label {
		opacity: 0.7;
	}
	.reset {
		flex: 0 0 auto;
		background: none;
		border: none;
		cursor: pointer;
		padding: 0 var(--spacing-xs);
		opacity: 0.6;
	}
	.reset:hover {
		opacity: 1;
	}
	.slider {
		width: 100%;
	}
</style>
