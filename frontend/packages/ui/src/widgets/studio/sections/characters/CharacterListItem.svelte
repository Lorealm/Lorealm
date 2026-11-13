<script lang="ts">
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import PopupMenu from '#lib/components/PopupMenu.svelte';
	import TransportImage from '#lib/components/TransportImage.svelte';
	import { blobUrl } from '#lib/api/blobs.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { CharacterSummary, ExportFormat, ExportProfileInfo } from '#lib/types/character.js';
	import {
		Copy,
		Ellipsis,
		FileImage,
		FileBraces,
		MessageSquarePlus,
		Pencil,
		Trash2,
		Upload
	} from '@lucide/svelte';
	import { useApi, usePlatform } from '#lib/api/client.js';
	import { showPopup } from '#lib/stores/popup.svelte.js';
	import { relativeTime } from '#lib/assets/scripts/datetime.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { showApiError } from '#lib/stores/errorActions.js';

	let { summary }: { summary: CharacterSummary } = $props();
	const relative = $derived(relativeTime(summary.updated_at) ?? '');
	let imageEndpoint = $derived(blobUrl(summary.avatar_uri));

	$effect(() => {
		void characterStore.loadExportProfiles().catch(() => {});
	});

	const exportItems = $derived(
		characterStore.exportProfiles.flatMap((profile) =>
			(['png', 'json'] as const).map((format) => ({
				label: `${profile.label} · ${format.toUpperCase()}`,
				icon: format === 'png' ? FileImage : FileBraces,
				onSelect: () => void exportAs(profile, format)
			}))
		)
	);

	/** The file name the server uses too: unsafe characters become `_`. */
	function fileStem(name: string): string {
		const stem = name.replace(/[^A-Za-z0-9 _-]/g, '_').trim();
		return stem || 'character';
	}

	async function exportAs(profile: ExportProfileInfo, format: ExportFormat) {
		const api = useApi();
		try {
			const warnings = await api.characters.exportWarnings(summary.id, profile.id, format);
			if (warnings.length > 0) {
				const choice = await showPopup({
					title: `Export ${summary.name} as ${profile.label}`,
					message: [
						profile.description,
						"**What won't carry over:**",
						warnings.map((w) => `- ${w.message}`).join('\n')
					].join('\n\n'),
					variant: 'warning',
					actions: [{ label: 'Cancel' }, { label: 'Download', tone: 'primary' }]
				});
				if (choice !== 'Download') return;
			}
			const file = await api.characters.export(summary.id, profile.id, format);
			await usePlatform().files.save(file, {
				label: format === 'png' ? 'Character card' : 'Character card (JSON)',
				extensions: [format],
				suggestedName: `${fileStem(summary.name)}.${format}`
			});
		} catch (e) {
			showApiError(e, { title: 'Could not export character' });
		}
	}

	let confirmingDelete = $state(false);
	let deleting = $state(false);

	async function remove() {
		deleting = true;
		try {
			await characterStore.delete(summary.id);
			chatStore.forgetCharacter(summary.id);
		} catch (e) {
			showApiError(e, { title: 'Could not delete character' });
			deleting = false;
			confirmingDelete = false;
		}
	}
</script>

<article class="row">
	<span class="portrait">
		<TransportImage src={imageEndpoint} alt={summary.name} />
	</span>

	<div class="details">
		<div class="headerRow">
			<h4 class="name" {@attach tooltip(summary.name)}>{summary.name}</h4>

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
			<div class="subheader" {@attach tooltip(summary.updated_at)}>Updated {relative}</div>
		{/if}
	</div>

	{#if confirmingDelete}
		<div class="controls confirm">
			<span class="confirmText">Delete {summary.name} and all of its chats?</span>
			<div class="confirmButtons">
				<button type="button" class="danger" disabled={deleting} onclick={remove}>Delete</button>
				<button type="button" disabled={deleting} onclick={() => (confirmingDelete = false)}>
					Cancel
				</button>
			</div>
		</div>
	{:else}
		<div class="controls">
			<button
				type="button"
				class="edit"
				onclick={() => navigator.start().studio().characters(summary.id)}
			>
				<Pencil size={16} />
				Edit
			</button>

			<div class="minorControls">
				<PopupMenu title="Export" placement="down" items={exportItems}>
					{#snippet icon()}<Upload size={16} />{/snippet}
				</PopupMenu>
				<button
					type="button"
					class="minor del"
					{@attach tooltip('Delete character')}
					aria-label={`Delete ${summary.name}`}
					onclick={() => (confirmingDelete = true)}
				>
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
	{/if}
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

	.confirm {
		max-width: 14em;
	}
	.confirmText {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.confirmButtons {
		display: flex;
		gap: var(--spacing-xs);
	}
	.confirmButtons > button {
		flex: 1 1 0;
	}
	.danger {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.danger:not(:disabled):hover {
		background-color: var(--color-error);
		color: var(--color-text-white);
	}
</style>
