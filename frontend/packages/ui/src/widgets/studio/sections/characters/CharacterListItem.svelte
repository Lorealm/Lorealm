<script lang="ts">
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import type { CharacterSummary } from '$lib/types/character.js';
	import { Pencil } from '@lucide/svelte';
	import { DateTime } from 'luxon';

	let { summary }: { summary: CharacterSummary } = $props();
	const relative = $derived(
		DateTime.fromISO(summary.updated_at).toLocaleString(DateTime.DATETIME_MED)
	);
	let imageEndpoint = $derived(`/api/v1/characters/${summary.id}/image`);
</script>

<div class="listItem">
	<span>
		<TransportImage src={imageEndpoint} alt={summary.name} />
	</span>
	<div class="titleSection">
		<div class="headerRow">
			<h2 class="header">{summary.name}</h2>

			{#if summary.tags && summary.tags.length > 0}
				<div class="tagContainer">
					<span class="tagBadge">{summary.tags[0]}</span>
					{#if summary.tags.length > 1}
						<span class="tagCount">+{summary.tags.length - 1}</span>
					{/if}
				</div>
			{/if}
		</div>

		<div class="subheader">Updated at: {relative}</div>
	</div>
	<button
		class="editBtn"
		onclick={() => {
			characterStore.startEditing(summary.id);
			studioStore.open(summary.id);
		}}><Pencil /> Edit</button
	>
</div>

<style>
	.listItem {
		display: flex;
		width: 100%;
		padding: var(--spacing-sm);
		flex-direction: row;
		height: 7em;
		background-color: var(--color-bg-elevated);
		gap: var(--spacing-lg);
	}
	.titleSection {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--spacing-sm);
	}
	.listItem > span {
		display: block;
		height: 100%;
		width: auto;
		aspect-ratio: 9 / 14; /* through trial and error */
		flex: none;
	}
	.listItem > span :global(img) {
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-md);
	}
	.header {
		color: var(--color-primary);
		margin: 0;
	}
	.subheader {
		color: var(--color-text-secondary);
	}
	.editBtn {
		display: flex;
		aspect-ratio: 1 / 1;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-md);
		margin-left: auto;
	}
	.headerRow {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-md);
	}

	.tagContainer {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs, 4px);
		font-size: 0.75rem; /* Small and sharp */
		font-weight: 500;
	}

	.tagBadge {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-primary);
		padding: 2px 8px;
		border-radius: 12px; /* Pill shape */
		text-transform: capitalize;
	}

	.tagCount {
		color: var(--color-text-secondary);
		background-color: transparent;
		border: 1px solid var(--color-bg-elevated-3);
		padding: 1px 6px;
		border-radius: 10px;
	}
</style>
