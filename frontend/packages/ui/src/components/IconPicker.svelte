<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import { iconFor, iconNames } from '#lib/assets/icons/catalogue.js';

	let { icon, onpick }: { icon: string; onpick: (name: string) => void } = $props();

	let open = $state(false);
	let query = $state('');
	let pickerEl = $state<HTMLElement>();
	let numIcons = iconNames.length;

	const Current = $derived(iconFor(icon));
	const matches = $derived(
		query ? iconNames.filter((n) => n.includes(query.toLowerCase().trim())) : iconNames
	);

	function pick(name: string) {
		onpick(name);
		open = false;
		query = '';
	}

	function onWindowPointerDown(e: PointerEvent) {
		if (open && !pickerEl?.contains(e.target as Node)) open = false;
	}
</script>

<svelte:window
	onpointerdown={onWindowPointerDown}
	onkeydown={(e) => e.key === 'Escape' && (open = false)}
/>

<span class="picker" bind:this={pickerEl}>
	<button
		type="button"
		class="trigger"
		{@attach tooltip(t.components.iconPicker.change)}
		aria-haspopup="dialog"
		aria-expanded={open}
		onclick={() => (open = !open)}
	>
		<Current size={16} />
	</button>

	{#if open}
		<div class="panel" role="dialog" aria-label={t.components.iconPicker.pick}>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				class="search"
				placeholder={t.components.iconPicker.search(numIcons)}
				bind:value={query}
				autofocus
			/>
			<div class="grid">
				{#each matches as name (name)}
					{@const Icon = iconFor(name)}
					<button
						type="button"
						class="option"
						class:selected={name === icon}
						{@attach tooltip(name)}
						onclick={() => pick(name)}
					>
						<Icon size={16} />
					</button>
				{/each}
				{#if matches.length === 0}
					<span class="empty">{t.components.iconPicker.noMatch(query)}</span>
				{/if}
			</div>
		</div>
	{/if}
</span>

<style>
	.picker {
		position: relative;
		display: inline-flex;
	}
	.trigger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		border-radius: var(--radius-md);
	}
	.panel {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		width: 16em;
		padding: var(--spacing-sm);
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
	}
	.search {
		width: 100%;
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-medium);
		font-size: var(--font-size-sm);
		padding: 6px var(--spacing-sm);
	}
	.search:focus {
		border-color: var(--color-border-active);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(26px, 1fr));
		gap: var(--spacing-xs);
		max-height: 14em;
		overflow-y: auto;
		overflow-x: hidden;
	}
	.option {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 4px;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-secondary);
	}
	.option:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.option.selected {
		background-color: var(--color-active);
		color: var(--color-text-accent);
	}
	.empty {
		grid-column: 1 / -1;
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
