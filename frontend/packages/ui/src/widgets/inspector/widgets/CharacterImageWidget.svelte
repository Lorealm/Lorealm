<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import TransportImage from '#lib/components/TransportImage.svelte';
	import { blobUrl } from '#lib/api/blobs.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { ChevronDown, Crop, Scan, UnfoldVertical } from '@lucide/svelte';

	const MODES = [
		{ id: 'cover', icon: Crop },
		{ id: 'fit', icon: Scan },
		{ id: 'full', icon: UnfoldVertical }
	] as const;

	type Mode = (typeof MODES)[number]['id'];

	let activeCharacter = $derived(chatStore.characterId);
	let imageEndpoint = $derived(blobUrl(characterStore.activeCharacter?.metadata.avatar_uri));
	let collapsed = $state(false);
	let mode = $state<Mode>('cover');

	function cycle() {
		const next = MODES.findIndex((m) => m.id === mode) + 1;
		mode = MODES[next % MODES.length].id;
	}
</script>

<div class="widget" class:collapsed class:fit={mode === 'fit'} class:full={mode === 'full'}>
	<div class="head">
		<button class="title" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
			<span class="chev" class:collapsed><ChevronDown /></span>
			<span class="label">{t.inspector.widgets.characterImage}</span>
		</button>
		{#if !collapsed && activeCharacter}
			<div class="modes">
				{#each MODES as option (option.id)}
					{@const Icon = option.icon}
					<button
						class="mode"
						class:active={option.id === mode}
						{@attach tooltip(t.inspector.image[option.id])}
						aria-label={t.inspector.image[option.id]}
						aria-pressed={option.id === mode}
						onclick={() => (mode = option.id)}
					>
						<Icon />
					</button>
				{/each}
			</div>
		{/if}
	</div>
	{#if !collapsed}
		{#if activeCharacter}
			<button
				class="image-container"
				{@attach tooltip(t.inspector.image.change)}
				aria-label={t.inspector.image.change}
				onclick={cycle}
			>
				<TransportImage src={imageEndpoint} />
			</button>
		{:else}
			<div class="empty">{t.inspector.image.noCharacter}</div>
		{/if}
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		overflow: hidden;
		height: var(--inspector-widget-large, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		padding-right: var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.title {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 1;
		min-width: 0;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		cursor: pointer;
	}
	.label {
		flex: 1;
		min-width: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev :global(svg) {
		width: 14px;
		height: 14px;
	}

	.modes {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		gap: 2px;
	}
	.mode {
		display: inline-flex;
		padding: 4px;
		border-radius: var(--radius-sm);
		background-color: transparent;
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.mode:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.mode.active {
		background-color: var(--color-active);
		color: var(--color-text-accent);
	}
	.mode :global(svg) {
		width: 13px;
		height: 13px;
	}

	.empty {
		margin: auto;
		padding: var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-align: center;
	}
	.image-container {
		display: block;
		position: relative;
		flex: 1 1 auto;
		min-height: 0;
		min-width: 0;
		overflow: hidden;
		padding: 0;
		border: none;
		border-radius: 0;
		background-color: transparent;
		cursor: pointer;
	}

	.image-container :global(.transport-image) {
		position: absolute;
		inset: 0;
	}

	.widget.fit .image-container :global(.transport-image img) {
		object-fit: contain;
	}

	.widget.full {
		height: auto;
	}
	.widget.full .image-container {
		flex: 0 0 auto;
	}
	.widget.full .image-container :global(.transport-image) {
		position: static;
		display: block;
	}
	.widget.full .image-container :global(.transport-image img) {
		width: 100%;
		height: auto;
		object-fit: fill;
	}

	.widget.full .image-container :global(.transport-image:not([data-status='loaded'])) {
		height: var(--inspector-widget-large, 280px);
	}
</style>
