<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	let {
		items,
		label,
		placeholder,
		onchange
	}: {
		items: string[];
		label: string;
		placeholder?: string;
		onchange: () => void;
	} = $props();

	let entry = $state('');

	function add() {
		const value = entry.trim();
		entry = '';
		if (!value || items.includes(value)) return;
		items.push(value);
		onchange();
	}

	function remove(index: number) {
		items.splice(index, 1);
		onchange();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			add();
		} else if (e.key === 'Backspace' && entry === '' && items.length > 0) {
			remove(items.length - 1);
		}
	}
</script>

<div class="tag-field">
	<label for="chips">{label}</label>
	<div class="chips">
		{#each items as item, i (item)}
			<span class="chip">
				{item}
				<button
					type="button"
					class="chip-x"
					aria-label={t.studio.tagField.remove(item)}
					onclick={() => remove(i)}>×</button
				>
			</span>
		{/each}
		<input
			class="chip-input"
			placeholder={placeholder ?? t.studio.tagField.add}
			bind:value={entry}
			onkeydown={onKeydown}
			onblur={add}
		/>
	</div>
</div>

<style>
	.tag-field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		background-color: var(--color-bg-elevated);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		padding: var(--spacing-xs);
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		background-color: var(--color-selected);
		color: var(--color-text-primary);
		border-radius: var(--radius-full);
		padding: 2px var(--spacing-sm);
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
		white-space: nowrap;
	}
	.chip-x {
		display: inline-flex;
		padding: 0;
		width: 14px;
		height: 14px;
		align-items: center;
		justify-content: center;
		background: transparent;
		border-radius: var(--radius-full);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		line-height: 1;
	}
	.chip-x:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.chip-input {
		flex: 1 1 80px;
		width: auto;
		min-width: 80px;
		background: transparent;
		padding: 2px var(--spacing-xs);
	}
</style>
