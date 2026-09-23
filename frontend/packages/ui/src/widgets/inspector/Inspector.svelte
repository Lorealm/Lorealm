<script lang="ts">
	import { iconFor } from '$lib/assets/icons/catalogue.js';
	import { router } from '$lib/navigation/router.svelte.js';
	import { inspectorMatrix } from '$lib/stores/inspectorMatrix.svelte.js';
	import { lazy } from '$lib/core/lazy.js';
	import { realmInspector } from '$lib/widgets/realms/lazyInspector.js';
	import { inspectorRegistry, isPluginWidgetKey } from './registry.js';
	import PluginWidget from './widgets/PluginWidget.svelte';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import { inspectorWidth } from '$lib/stores/inspectorWidth.svelte.js';

	const view = $derived(router.current.view);

	$effect(() => {
		void inspectorWidth.load();
		void pluginsStore.load();
	});

	function startResize(event: PointerEvent) {
		event.preventDefault();
		const startX = event.clientX;
		const startWidth = inspectorWidth.value;
		const move = (e: PointerEvent) => inspectorWidth.set(startWidth + (startX - e.clientX));
		const stop = () => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', stop);
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', stop);
	}

	function nudge(event: KeyboardEvent) {
		const step = event.shiftKey ? 48 : 16;
		if (event.key === 'ArrowLeft') inspectorWidth.set(inspectorWidth.value + step);
		else if (event.key === 'ArrowRight') inspectorWidth.set(inspectorWidth.value - step);
		else return;
		event.preventDefault();
	}
	const profiles = $derived(inspectorMatrix.ordered(view));
	const widgets = $derived(inspectorMatrix.widgetsFor(view));
</script>

<div class="container" style="width: {inspectorWidth.value}px">
	<button
		type="button"
		class="grip"
		aria-label="Resize inspector, currently {inspectorWidth.value} pixels. Use arrow keys."
		onpointerdown={startResize}
		onkeydown={nudge}
	></button>
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
			{#await lazy(realmInspector) then RealmInspector}
				<RealmInspector />
			{/await}
		{:else}
			{#each widgets as key (key)}
				{#if isPluginWidgetKey(key)}
					{@const slot = pluginsStore.inspectorSlot(key)}
					{#if slot}<PluginWidget {slot} />{/if}
				{:else}
					{@const Widget = inspectorRegistry[key].component}
					<Widget />
				{/if}
			{:else}
				<span class="empty">Nothing configured for this view.</span>
			{/each}
		{/if}
	</div>
</div>

<style>
	.container {
		position: relative;
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		height: 100%;
		min-width: 0;
		background-color: var(--color-bg-elevated);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
		border-left: 1px solid var(--color-border-subtle);
	}
	.grip {
		position: absolute;
		top: 0;
		left: -3px;
		width: 6px;
		height: 100%;
		z-index: 2;
		padding: 0;
		border: none;
		cursor: col-resize;
		background: transparent;
	}
	.grip:hover,
	.grip:focus-visible {
		background-color: var(--color-border-active);
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
