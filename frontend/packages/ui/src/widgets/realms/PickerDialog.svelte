<script lang="ts">
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { ImportKind } from '$lib/types/realm.js';
	import { SvelteSet } from 'svelte/reactivity';
	import { catalogItems, IMPORT_LABEL, realmEditor } from './realmEditor.svelte.js';

	let { kind }: { kind: ImportKind } = $props();

	let query = $state('');
	const selected = new SvelteSet<Snowflake>();
	let busy = $state(false);

	const imports = $derived(realmStore.realm?.definition.imports ?? []);
	const already = $derived(
		new Set(imports.filter((r) => r.kind === kind).map((r) => r.resource_id))
	);
	const candidates = $derived(catalogItems(kind).filter((c) => !already.has(c.id)));
	const results = $derived(
		candidates.filter(
			(c) => !query.trim() || c.name.toLowerCase().includes(query.trim().toLowerCase())
		)
	);

	function close() {
		realmEditor.pickerKind = null;
	}

	async function confirm() {
		if (busy || selected.size === 0) {
			close();
			return;
		}
		busy = true;
		try {
			const added = [...selected].map((id) => ({ kind, resource_id: id }));
			await realmStore.setImports([...imports, ...added]);
			showToast({
				variant: 'success',
				title: 'Added to realm',
				message: `${added.length} ${IMPORT_LABEL[kind].toLowerCase()} imported.`
			});
			close();
		} finally {
			busy = false;
		}
	}

	$effect(() => {
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') close();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="overlay"
	onclick={(e) => {
		if (e.target === e.currentTarget) close();
	}}
>
	<div class="panel" role="dialog" aria-modal="true" aria-label="Add {IMPORT_LABEL[kind]}">
		<header><h3>Add {IMPORT_LABEL[kind]}</h3></header>

		<input type="text" bind:value={query} placeholder="Search…" />

		<div class="list">
			{#each results as item (item.id)}
				<button
					type="button"
					class="row"
					class:on={selected.has(item.id)}
					onclick={() => (selected.has(item.id) ? selected.delete(item.id) : selected.add(item.id))}
				>
					<input type="checkbox" checked={selected.has(item.id)} tabindex="-1" />
					<div class="rowBody">
						<div class="name">{item.name}</div>
						{#if item.description}<div class="desc">{item.description}</div>{/if}
					</div>
				</button>
			{:else}
				{#if candidates.length === 0}
					<div class="none">Everything of this kind is already imported.</div>
				{:else}
					<div class="none">No matches.</div>
				{/if}
			{/each}
		</div>

		<footer>
			<button type="button" onclick={close}>Cancel</button>
			<button type="button" class="primary" onclick={confirm} disabled={busy}>
				{selected.size ? `Add (${selected.size})` : 'Add'}
			</button>
		</footer>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal, 100);
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.5);
		padding: var(--spacing-lg);
	}

	.panel {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: min(460px, 100%);
		max-height: 80vh;
		padding: var(--spacing-lg);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-md);
	}

	header h3 {
		margin: 0;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		max-height: 280px;
		overflow-y: auto;
	}

	.row {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm);
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		text-align: left;
	}

	.row:hover {
		background: var(--color-bg-elevated-2);
	}

	.row.on {
		background: var(--color-selected);
	}

	.row input {
		width: auto;
		margin-top: 2px;
		pointer-events: none;
	}

	.rowBody {
		min-width: 0;
	}

	.name {
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
	}

	.desc {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.none {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}

	footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-sm);
		border-top: 1px solid var(--color-border-subtle);
		padding-top: var(--spacing-lg);
	}

	.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
</style>
