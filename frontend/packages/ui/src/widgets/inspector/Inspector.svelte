<script lang="ts">
	import { iconFor } from '$lib/assets/icons/catalogue.js';
	import { router } from '$lib/navigation/router.svelte.js';
	import { inspectorMatrix } from '$lib/stores/inspectorMatrix.svelte.js';
	import RealmInspector from '../realms/RealmInspector.svelte';
	import { inspectorRegistry } from './registry.js';

	const view = $derived(router.current.view);
	const profiles = $derived(inspectorMatrix.ordered(view));
	const widgets = $derived(inspectorMatrix.widgetsFor(view));
</script>

<div class="container">
	<div class="topBar">
		{#if view === 'realms'}
			<button>Realm Placeholder</button>
		{:else}
			{#each profiles as profile (profile.id)}
				{@const Icon = iconFor(profile.icon)}
				{@const active = profile.id === inspectorMatrix.activeId}
				<button
					class="profile"
					class:active
					class:unconfigured={!inspectorMatrix.isConfigured(profile, view)}
					title={profile.name}
					aria-pressed={active}
					onclick={() => inspectorMatrix.select(profile.id)}
				>
					<Icon size={16} />
					{#if active}<span class="name">{profile.name}</span>{/if}
				</button>
			{/each}
		{/if}
	</div>
	<div class="content">
		{#if view === 'realms'}
			<RealmInspector />
		{:else}
			{#each widgets as key (key)}
				{@const Widget = inspectorRegistry[key].component}
				<Widget />
			{:else}
				<span class="empty">Nothing configured for this view.</span>
			{/each}
		{/if}
	</div>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--width-sidebar-small);
		background-color: var(--color-bg-elevated);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
		border-left: 1px solid var(--color-border-subtle);
	}
	.topBar {
		display: flex;
		flex-shrink: 0;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		width: 100%;
		height: var(--height-header);
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
		overflow-x: auto;
	}
	.profile {
		display: inline-flex;
		flex-shrink: 0;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 4px 5px;
		border-radius: var(--radius-full);
		background: transparent;
		color: var(--color-text-secondary);
	}
	.profile.unconfigured {
		color: var(--color-text-disabled);
	}
	.profile:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.profile.active {
		padding-right: var(--spacing-sm);
		background-color: var(--color-active);
		color: var(--color-text-accent);
	}
	.profile.active.unconfigured {
		color: var(--color-text-secondary);
	}
	.name {
		font-size: var(--font-size-sm);
		line-height: 1;
		max-width: 9em;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.content {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	.empty {
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
