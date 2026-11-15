<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { marketplaceStore, shownVersion } from '#lib/stores/marketplace.svelte.js';
	import { pluginsStore } from '#lib/stores/plugins.svelte.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import type { ListedPlugin } from '#lib/types/plugins.js';
	import { canInstall } from './actions.svelte.js';

	let query = $state('');

	onMount(() => {
		void marketplaceStore.load();
		void pluginsStore.load();
		void configStore.loadPermissions();
	});

	const shown = $derived.by(() => {
		const words = query.toLowerCase().split(/\s+/).filter(Boolean);
		return marketplaceStore.plugins.filter((plugin) => {
			const version = shownVersion(plugin);
			const text = [version.name, version.description, version.author, ...version.tags]
				.join(' ')
				.toLowerCase();
			return words.every((word) => text.includes(word));
		});
	});

	function status(plugin: ListedPlugin): { label: string; kind: 'on' | 'note' | 'off' } | null {
		if (!plugin.offered) return { label: t.plugins.status.unsupported, kind: 'off' };
		if (canInstall() && marketplaceStore.updateFor(plugin.handle)) {
			return { label: t.plugins.status.update, kind: 'note' };
		}
		const mine = pluginsStore.plugins.find((p) => p.handle === plugin.handle);
		if (mine?.enabled) return { label: t.plugins.status.enabled, kind: 'on' };
		if (mine) return { label: t.plugins.status.installed, kind: 'note' };
		return null;
	}
</script>

<div class="bar">
	<input
		type="search"
		bind:value={query}
		placeholder={t.plugins.search}
		aria-label={t.plugins.search}
	/>
</div>

{#if marketplaceStore.unavailable.length}
	<p class="notice">
		{t.plugins.sourcesMissing({ count: marketplaceStore.unavailable.length })}
	</p>
{/if}

{#if !marketplaceStore.loaded}
	<div class="empty">{t.plugins.loading}</div>
{:else if marketplaceStore.failed}
	<div class="empty">{t.plugins.marketplaceFailed}</div>
{:else if marketplaceStore.plugins.length === 0}
	<div class="empty">{t.plugins.marketplaceEmpty}</div>
{:else if shown.length === 0}
	<div class="empty">{t.plugins.noMatch({ query })}</div>
{:else}
	<div class="grid">
		{#each shown as plugin (plugin.handle)}
			{@const version = shownVersion(plugin)}
			{@const state = status(plugin)}
			<button
				type="button"
				class="card"
				class:unavailable={!plugin.offered}
				onclick={() => navigator.start().marketplace(plugin.handle)}
			>
				<span class="title">
					<span class="name">{version.name}</span>
					<span class="muted">{version.version}</span>
				</span>
				<span class="muted">{version.author}</span>
				{#if version.description}<span class="description">{version.description}</span>{/if}
				<span class="foot">
					{#each version.tags.slice(0, 3) as tag (tag)}<span class="tag">{tag}</span>{/each}
					{#if state}<span class="status {state.kind}">{state.label}</span>{/if}
				</span>
			</button>
		{/each}
	</div>
{/if}

<style>
	.bar {
		flex: 0 0 auto;
		padding: var(--spacing-md) var(--spacing-lg) 0;
	}
	.bar input {
		width: 100%;
		max-width: 420px;
	}
	.notice {
		flex: 0 0 auto;
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-lg) 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.grid {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		align-content: start;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}
	.card {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--spacing-xs);
		min-width: 0;
		padding: var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated);
		text-align: left;
		cursor: pointer;
	}
	.card:hover {
		border-color: var(--color-border-medium);
	}
	.card.unavailable {
		opacity: 0.6;
	}
	.title {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		max-width: 100%;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
	}
	.muted {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.description {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		overflow: hidden;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.foot {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		width: 100%;
		margin-top: auto;
		padding-top: var(--spacing-xs);
	}
	.tag {
		padding: 0 var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.status {
		margin-left: auto;
		font-size: var(--font-size-xs);
		color: var(--color-text-secondary);
	}
	.status.on {
		color: var(--color-success-text);
	}
	.status.off {
		color: var(--color-text-muted);
	}

	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
	}
</style>
