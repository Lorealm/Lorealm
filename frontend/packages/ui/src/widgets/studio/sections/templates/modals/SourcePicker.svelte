<script lang="ts">
	import type { SourceDefinition, SourceNodeDefinition } from '$lib/types/node.js';

	let {
		sources,
		existing,
		onpick
	}: {
		sources: SourceDefinition[] | undefined;
		existing: Set<string>;
		onpick: (sel: { source_id: string; node_id: string; def: SourceNodeDefinition }) => void;
	} = $props();

	const key = (sourceId: string, nodeId: string) => `${sourceId}\u0000${nodeId}`;

	// Each source paired with its not-yet-used nodes.
	const available = $derived.by(() => {
		const list = sources ?? [];
		return list
			.map((s) => ({
				source: s,
				nodes: s.nodes.filter((n) => !existing.has(key(s.source_id, n.node_id)))
			}))
			.filter((entry) => entry.nodes.length > 0);
	});

	let selectedSourceId = $state<string | null>(null);
	const current = $derived(
		selectedSourceId
			? (available.find((e) => e.source.source_id === selectedSourceId) ?? null)
			: null
	);
</script>

<div class="source-picker">
	{#if sources === undefined}
		<p class="muted">Loading sources…</p>
	{:else if available.length === 0}
		<p class="muted">Every available source node is already in this template.</p>
	{:else if !current}
		<ul class="list">
			{#each available as entry (entry.source.source_id)}
				<li>
					<button
						class="row"
						type="button"
						onclick={() => (selectedSourceId = entry.source.source_id)}
					>
						<span class="row-main">
							<span class="row-title">{entry.source.name}</span>
							{#if entry.source.description}
								<span class="row-desc">{entry.source.description}</span>
							{/if}
						</span>
						<span class="count">{entry.nodes.length}</span>
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		<button class="back" type="button" onclick={() => (selectedSourceId = null)}>
			← {current.source.name}
		</button>
		<ul class="list">
			{#each current.nodes as node (node.node_id)}
				<li>
					<button
						class="row"
						type="button"
						onclick={() =>
							onpick({
								source_id: current.source.source_id,
								node_id: node.node_id,
								def: node
							})}
					>
						<span class="row-main">
							<span class="row-title">{node.title}</span>
							{#if node.description}
								<span class="row-desc">{node.description}</span>
							{/if}
						</span>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
</div>

<style>
	.source-picker {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}
	.muted {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.row {
		width: 100%;
		justify-content: space-between;
		align-items: center;
		text-align: left;
		gap: var(--spacing-sm);
	}
	.row:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.row-main {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.row-title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row-desc {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.count {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.back {
		align-self: flex-start;
		background-color: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.back:hover {
		color: var(--color-text-accent);
	}
</style>
