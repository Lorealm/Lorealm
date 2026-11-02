<script lang="ts">
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import IconPicker from '$lib/components/IconPicker.svelte';
	import type { ViewName } from '$lib/navigation/routes.js';
	import { inspectorMatrix } from '$lib/stores/inspectorMatrix.svelte.js';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import {
		inspectorRegistry,
		inspectorWidgetKeys,
		isPluginWidgetKey,
		type InspectorWidgetKey
	} from '$lib/widgets/inspector/registry.js';
	import { ChevronDown, ChevronUp, Plus, Search, Trash2, X } from '@lucide/svelte';

	const VIEWS: Array<{ id: ViewName; label: string }> = [
		{ id: 'write', label: 'Write' },
		{ id: 'characters', label: 'Characters' },
		{ id: 'realms', label: 'Realms' },
		{ id: 'connections', label: 'Connections' },
		{ id: 'studio', label: 'Studio' },
		{ id: 'plugins', label: 'Plugins' },
		{ id: 'settings', label: 'Settings' }
	];

	let selectedId = $state<string | null>(null);
	let view = $state<ViewName>('write');
	let query = $state('');

	const profile = $derived(
		inspectorMatrix.profiles.find((p) => p.id === selectedId) ?? inspectorMatrix.profiles[0]
	);
	const stack = $derived(profile?.cells[view] ?? []);

	$effect(() => {
		void pluginsStore.load();
	});

	/** A plugin's widget keeps its key in a profile after the plugin is disabled. */
	function label(key: InspectorWidgetKey): string {
		if (!isPluginWidgetKey(key)) return inspectorRegistry[key].label;
		return pluginsStore.inspectorSlot(key)?.title ?? `${key} (unavailable)`;
	}

	const available = $derived.by(() => {
		const q = query.toLowerCase().trim();
		const keys: InspectorWidgetKey[] = [
			...inspectorWidgetKeys,
			...pluginsStore.slots('inspector').map((s) => s.key)
		];
		return keys
			.filter((key) => !stack.includes(key))
			.filter((key) => !q || label(key).toLowerCase().includes(q));
	});

	async function addProfile() {
		selectedId = await inspectorMatrix.addProfile();
	}
</script>

<section class="section">
	<div class="body">
		<h2>Inspector</h2>
		<p class="hint">
			A profile is one column of the inspector: it decides which widgets each view shows, and in
			what order. Profiles appear as icon buttons above the inspector; ones with nothing configured
			for the current view are greyed out and sorted last.
		</p>

		<div class="profiles">
			{#each inspectorMatrix.profiles as p (p.id)}
				<button
					class="chip"
					class:active={p.id === profile?.id}
					onclick={() => (selectedId = p.id)}
				>
					{p.name}
				</button>
			{/each}
			<button class="chip add" {@attach tooltip('Add profile')} onclick={addProfile}>
				<Plus size={14} />
			</button>
		</div>

		{#if profile}
			<div class="header">
				<IconPicker
					icon={profile.icon}
					onpick={(name) => inspectorMatrix.setIcon(profile.id, name)}
				/>
				<input
					class="name"
					value={profile.name}
					oninput={(e) => inspectorMatrix.rename(profile.id, e.currentTarget.value)}
				/>
				<button
					class="delete"
					{@attach tooltip('Delete profile')}
					disabled={inspectorMatrix.profiles.length < 2}
					onclick={() => inspectorMatrix.removeProfile(profile.id)}
				>
					<Trash2 size={16} />
				</button>
			</div>

			<div class="views">
				{#each VIEWS as v (v.id)}
					{@const count = (profile.cells[v.id] ?? []).length}
					<button class="viewTab" class:active={v.id === view} onclick={() => (view = v.id)}>
						{v.label}
						{#if count > 0}<span class="count">{count}</span>{/if}
					</button>
				{/each}
			</div>

			<div class="stack">
				{#each stack as key, i (key)}
					<div class="entry">
						<span class="entryLabel">{label(key)}</span>
						<button
							class="iconBtn"
							{@attach tooltip('Move up')}
							disabled={i === 0}
							onclick={() => inspectorMatrix.moveWidget(profile.id, view, i, -1)}
						>
							<ChevronUp size={14} />
						</button>
						<button
							class="iconBtn"
							{@attach tooltip('Move down')}
							disabled={i === stack.length - 1}
							onclick={() => inspectorMatrix.moveWidget(profile.id, view, i, 1)}
						>
							<ChevronDown size={14} />
						</button>
						<button
							class="iconBtn"
							{@attach tooltip('Remove')}
							onclick={() => inspectorMatrix.removeWidget(profile.id, view, i)}
						>
							<X size={14} />
						</button>
					</div>
				{:else}
					<span class="empty">
						No widgets here yet — this profile is greyed out on {VIEWS.find((v) => v.id === view)
							?.label}.
					</span>
				{/each}
			</div>

			<div class="picker">
				<div class="searchRow">
					<Search size={14} />
					<input class="search" placeholder="Search widgets to add" bind:value={query} />
				</div>
				<div class="results">
					{#each available as key (key)}
						<button class="result" onclick={() => inspectorMatrix.addWidget(profile.id, view, key)}>
							<Plus size={13} />
							{label(key)}
						</button>
					{:else}
						<span class="empty">
							{query ? `Nothing matches “${query}”.` : 'Every widget is already in this stack.'}
						</span>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</section>

<style>
	.section {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		flex-direction: column;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
	}
	.body h2 {
		font-size: var(--font-size-h4);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		max-width: 46em;
		margin: 0;
	}
	.profiles {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
	}
	.chip {
		padding: 6px 14px;
		border-radius: var(--radius-full);
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
	}
	.chip.active {
		border-color: currentColor;
	}
	.chip.add {
		display: inline-flex;
		align-items: center;
		padding: 6px 10px;
	}
	.header {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.name {
		flex: 1 1 auto;
		max-width: 20em;
	}
	.delete {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		background: transparent;
		color: var(--color-text-secondary);
	}
	.delete:disabled {
		color: var(--color-text-disabled);
		cursor: default;
	}
	.views {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-xs);
		padding-bottom: var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.viewTab {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 4px 10px;
		border-radius: var(--radius-full);
		background: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.viewTab.active {
		background-color: var(--color-active);
		color: var(--color-text-accent);
	}
	.count {
		font-size: var(--font-size-xs);
		opacity: 0.7;
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		max-width: 32em;
	}
	.entry {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 6px var(--spacing-sm);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}
	.entryLabel {
		flex: 1 1 auto;
		font-size: var(--font-size-sm);
	}
	.iconBtn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 3px;
		background: transparent;
		color: var(--color-text-secondary);
	}
	.iconBtn:disabled {
		color: var(--color-text-disabled);
		cursor: default;
		background: transparent;
	}
	.picker {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		max-width: 32em;
	}
	.searchRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
	}
	.searchRow:focus-within {
		border-color: var(--color-border-active);
	}
	.search {
		background: transparent;
		border: none;
		padding: 6px 0;
		font-size: var(--font-size-sm);
	}
	.results {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-xs);
	}
	.result {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 4px 10px;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-2);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
