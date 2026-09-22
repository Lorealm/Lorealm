<script lang="ts">
	import { ChevronDown, Plus, Search } from '@lucide/svelte';
	import { useApi } from '$lib/api/client.js';
	import { blueprintStore } from '$lib/stores/blueprint.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import type { BlueprintSummary, InputSpec } from '$lib/types/blueprint.js';
	import InstantiateDialog from './InstantiateDialog.svelte';

	const api = useApi();

	let open = $state(false);
	let query = $state('');
	let rootEl: HTMLDivElement;
	let searchEl = $state<HTMLInputElement>();
	let picked = $state<BlueprintSummary | null>(null);
	let specs = $state<InputSpec[]>([]);
	let building = $state(false);

	const needle = $derived(query.trim().toLowerCase());
	const matches = $derived(
		blueprintStore.summaries.filter((b) => !needle || b.name.toLowerCase().includes(needle))
	);

	$effect(() => {
		searchEl?.focus();
	});

	$effect(() => {
		if (!open) return;
		const onDown = (e: PointerEvent) => {
			const target = e.target as Node | null;
			if (target && rootEl.contains(target)) return;
			open = false;
		};
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') open = false;
		};
		window.addEventListener('pointerdown', onDown, true);
		window.addEventListener('keydown', onKey);
		return () => {
			window.removeEventListener('pointerdown', onDown, true);
			window.removeEventListener('keydown', onKey);
		};
	});

	function toggle() {
		query = '';
		open = !open;
	}

	async function pick(blueprint: BlueprintSummary) {
		open = false;
		picked = blueprint;
		specs = await api.blueprints.inputs(blueprint.id).catch(() => []);
	}

	async function build(inputs: Record<string, unknown>) {
		if (!picked) return;
		building = true;
		try {
			const made = await blueprintStore.expand(picked.id, inputs);
			if (!made) return;
			picked = null;
			showToast({
				variant: 'info',
				title: 'Realm built',
				message: `${made.definition.name || 'It'} is ready to play.`
			});
			await realmStore.loadList();
		} finally {
			building = false;
		}
	}
</script>

<div class="root" bind:this={rootEl}>
	<button
		type="button"
		class="trigger"
		aria-haspopup="listbox"
		aria-expanded={open}
		onclick={toggle}
	>
		<Plus size={13} />
		<span class="label">New</span>
		<span class="caret"><ChevronDown size={13} /></span>
	</button>

	{#if open}
		<div class="panel">
			<div class="search">
				<Search size={13} />
				<input
					bind:this={searchEl}
					class="plain"
					type="text"
					bind:value={query}
					placeholder="Search blueprints…"
					aria-label="Search blueprints"
					autocomplete="off"
					spellcheck="false"
					onkeydown={(e) => e.key === 'Enter' && matches[0] && pick(matches[0])}
				/>
			</div>

			<div class="list" role="listbox" aria-label="Blueprints">
				{#each matches as blueprint (blueprint.id)}
					<button
						type="button"
						class="item"
						role="option"
						aria-selected="false"
						onclick={() => pick(blueprint)}
					>
						{blueprint.name}
					</button>
				{:else}
					<p class="empty">{needle ? 'No matching blueprints.' : 'No blueprints yet.'}</p>
				{/each}
			</div>
		</div>
	{/if}
</div>

{#if picked}
	<InstantiateDialog {specs} busy={building} onbuild={build} oncancel={() => (picked = null)} />
{/if}

<style>
	.root {
		position: relative;
	}
	.trigger {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.trigger:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.label {
		text-transform: uppercase;
	}
	.caret {
		display: flex;
		padding-left: var(--spacing-xs);
		margin-left: var(--spacing-xs);
		border-left: 1px solid var(--color-border-medium);
	}

	.panel {
		position: absolute;
		top: calc(100% + var(--spacing-xs));
		left: 0;
		z-index: var(--z-index-dropdown);
		display: flex;
		flex-direction: column;
		width: 240px;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}
	.search {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
	}
	.search input {
		flex: 1 1 auto;
		min-width: 0;
		background: transparent;
		border: none;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 240px;
		overflow-y: auto;
	}
	.item {
		width: 100%;
		padding: var(--spacing-xs) var(--spacing-sm);
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: left;
		background-color: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.item:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.empty {
		margin: 0;
		padding: var(--spacing-xs) var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
