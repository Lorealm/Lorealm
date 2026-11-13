<script lang="ts">
	import {
		SLOT_CHOICES,
		slotToForm,
		formToSlot,
		slotLabel,
		type SlotChoice
	} from '#lib/weaver/slotForm.js';
	import type { Slot } from '#lib/types/lorebook.js';
	import NumberInput from '#lib/components/NumberInput.svelte';

	let {
		value,
		onchange,
		allowInherit = true,
		inheritedDefault,
		ariaLabel
	}: {
		value: Slot | undefined;
		onchange: (next: Slot | undefined) => void;
		allowInherit?: boolean;
		inheritedDefault?: Slot;
		ariaLabel?: string;
	} = $props();

	const form = $derived(slotToForm(value));
	const choices = $derived(
		allowInherit ? SLOT_CHOICES : SLOT_CHOICES.filter((c) => c !== 'inherit')
	);

	function setChoice(choice: SlotChoice) {
		onchange(formToSlot({ choice, depth: form.depth }));
	}
	function setDepth(depth: number) {
		onchange(formToSlot({ choice: 'at_depth', depth: Math.max(0, Math.trunc(depth)) }));
	}

	function optionLabel(c: SlotChoice): string {
		if (c === 'inherit') {
			return inheritedDefault ? `inherit (${slotLabel(inheritedDefault)})` : 'inherit default';
		}
		return c;
	}
</script>

<div class="slot-picker">
	<select
		value={form.choice}
		aria-label={ariaLabel}
		onchange={(e) => setChoice(e.currentTarget.value as SlotChoice)}
	>
		{#each choices as c (c)}
			<option value={c}>{optionLabel(c)}</option>
		{/each}
	</select>
	{#if form.choice === 'at_depth'}
		<NumberInput
			min={0}
			step={1}
			width="5.5rem"
			ariaLabel="Depth"
			value={form.depth}
			onchange={(v) => setDepth(v === '' ? 0 : v)}
		/>
	{/if}
</div>

<style>
	.slot-picker {
		display: flex;
		gap: var(--spacing-xs);
		align-items: center;
	}
</style>
