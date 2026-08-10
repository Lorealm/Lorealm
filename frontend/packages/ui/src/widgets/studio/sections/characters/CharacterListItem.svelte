<script lang="ts">
	import PopupMenu from '$lib/components/PopupMenu.svelte';
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import type { CharacterSummary } from '$lib/types/character.js';
	import { Copy, Ellipsis, MessageSquarePlus, Pencil, Trash2, Upload } from '@lucide/svelte';
	import { DateTime } from 'luxon';

	let { summary }: { summary: CharacterSummary } = $props();
	const relative = $derived(DateTime.fromISO(summary.updated_at).toRelative() ?? '');
	let imageEndpoint = $derived(`/api/v1/characters/${summary.id}/image`);

	const unavailable = 'Not available yet';
</script>

<article class="row">
	<span class="portrait">
		<TransportImage src={imageEndpoint} alt={summary.name} />
	</span>

	<div class="details">
		<div class="headerRow">
			<h4 class="name" title={summary.name}>{summary.name}</h4>

			{#if summary.tags.length > 0}
				<div class="tagContainer">
					<span class="tagBadge">{summary.tags[0]}</span>
					{#if summary.tags.length > 1}
						<span class="tagCount">+{summary.tags.length - 1}</span>
					{/if}
				</div>
			{/if}
		</div>

		{#if relative}
			<div class="subheader" title={summary.updated_at}>Updated {relative}</div>
		{/if}
	</div>

	<div class="controls">
		<button
			type="button"
			class="edit"
			onclick={() => {
				characterStore.startEditing(summary.id);
				studioStore.open(summary.id);
			}}
		>
			<Pencil size={16} />
			Edit
		</button>

		<div class="minorControls">
			<button type="button" class="minor" title={unavailable} disabled aria-label="Export">
				<Upload size={16} />
			</button>
			<button type="button" class="minor del" title={unavailable} disabled aria-label="Delete">
				<Trash2 size={16} />
			</button>
			<PopupMenu
				title="More actions"
				placement="down"
				items={[
					{ label: 'Duplicate', icon: Copy, onSelect: () => {}, disabled: true },
					{ label: 'Start chat', icon: MessageSquarePlus, onSelect: () => {}, disabled: true }
				]}
			>
				{#snippet icon()}<Ellipsis size={16} />{/snippet}
			</PopupMenu>
		</div>
	</div>
</article>

<style>
	.row {
		display: flex;
		flex-direction: row;
		align-items: center;
		width: 100%;
		height: 7em;
		padding: var(--spacing-sm);
		gap: var(--spacing-lg);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		background-color: var(--color-bg-elevated);
		box-sizing: border-box;
	}
	.row:hover {
		border-color: var(--color-border-medium);
		box-shadow: var(--shadow-md);
	}

	.portrait {
		display: block;
		height: 100%;
		width: auto;
		aspect-ratio: 9 / 14; /* through trial and error */
		flex: none;
	}
	.portrait :global(img) {
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-md);
	}

	.details {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.name {
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-primary);
	}
	.subheader {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}

	.headerRow {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-md);
		min-width: 0;
	}

	.tagContainer {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-medium);
	}

	.tagBadge {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-primary);
		padding: 2px 8px;
		border-radius: var(--radius-full);
		text-transform: capitalize;
	}

	.tagCount {
		color: var(--color-text-secondary);
		background-color: transparent;
		border: 1px solid var(--color-bg-elevated-3);
		padding: 1px 6px;
		border-radius: var(--radius-full);
	}

	.controls {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--spacing-sm);
		margin-left: auto;
		flex: none;
	}
	.edit {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
	}

	.minorControls {
		display: flex;
		flex-direction: row;
		gap: var(--spacing-xs);
	}
	/* the three minor controls share the width of the Edit button above them */
	.minorControls > :global(*) {
		flex: 1 1 0;
	}
	.minor,
	.minorControls :global(.trigger) {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		padding: var(--spacing-sm) 0;
	}
	.minor:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.del:not(:disabled):hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
</style>
