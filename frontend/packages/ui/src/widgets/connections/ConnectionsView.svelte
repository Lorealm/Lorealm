<script lang="ts">
	import { onDestroy } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import { SECRETS } from '#lib/navigation/routes.js';
	import { connectionStore } from '#lib/stores/connection.svelte.js';
	import { PROVIDER_SCHEMAS } from '#lib/types/schema.js';
	import type { Snowflake } from '#lib/types/core.js';
	import ProfileList from './ProfileList.svelte';
	import ProfileEditor from './ProfileEditor.svelte';
	import SecretsManager from './SecretsManager.svelte';
	import ListSidebar from '#lib/components/ListSidebar.svelte';
	import SaveIndicator from '#lib/components/SaveIndicator.svelte';
	import { KeyRound } from '@lucide/svelte';

	const SAVE_MS = 500;

	const route = $derived(router.current);
	const selected = $derived(route.view === 'connections' ? (route.id ?? null) : null);
	const draft = $derived(connectionStore.editingProfile);

	let query = $state('');
	const profiles = $derived(connectionStore.profiles);
	const shownProfiles = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (!needle) return profiles ?? [];
		return (profiles ?? []).filter(
			(p) =>
				p.name.toLowerCase().includes(needle) ||
				p.provider.toLowerCase().includes(needle) ||
				p.model.toLowerCase().includes(needle)
		);
	});

	let timer: ReturnType<typeof setTimeout> | undefined;
	let dirty = false;

	function scheduleSave() {
		dirty = true;
		clearTimeout(timer);
		timer = setTimeout(() => {
			dirty = false;
			void connectionStore.saveProfile();
		}, SAVE_MS);
	}

	/** Commit any pending debounced edit before the draft is replaced/dropped. */
	async function flushPending() {
		if (!dirty) return;
		clearTimeout(timer);
		dirty = false;
		await connectionStore.saveProfile();
	}

	onDestroy(() => {
		clearTimeout(timer);
		void flushPending();
		connectionStore.stopEditing();
	});

	async function selectProfile(id: Snowflake) {
		if (selected === id) return;
		await flushPending();
		navigator.start().connections(id);
	}

	async function openSecrets() {
		await flushPending();
		navigator.start().connections(SECRETS);
	}

	async function activateProfile(id: Snowflake) {
		connectionStore.setActiveProfile(id);
	}

	let creating = $state(false);

	async function createProfile() {
		if (creating) return;
		creating = true;
		try {
			const id = await connectionStore.createProfile(
				'Untitled profile',
				PROVIDER_SCHEMAS[0]?.provider ?? ''
			);
			if (id) await selectProfile(id);
		} catch (e) {
			console.error(e);
		} finally {
			creating = false;
		}
	}

	async function deleteProfile(id: Snowflake) {
		if (selected === id) navigator.start().connections();
		try {
			await connectionStore.deleteProfile(id);
		} catch (e) {
			console.error(e);
		}
	}
</script>

<div class="connections-view">
	<ListSidebar
		title="Connections"
		createLabel="New profile"
		oncreate={createProfile}
		{creating}
		detailOpen={selected !== null}
		bind:query
		searchPlaceholder="Search profiles…"
	>
		{#snippet status()}
			<SaveIndicator state={connectionStore.saveState} />
		{/snippet}

		<button
			type="button"
			class="secrets-item"
			class:selected={selected === SECRETS}
			onclick={() => void openSecrets()}
		>
			<KeyRound size={14} />
			<span>API keys</span>
		</button>

		{#if shownProfiles.length === 0}
			<p class="empty">
				{(profiles ?? []).length === 0 ? 'No profiles yet.' : 'No profiles match that search.'}
			</p>
		{/if}
		<ProfileList
			profiles={shownProfiles}
			selectedId={selected === SECRETS ? null : selected}
			onActivate={(id) => void activateProfile(id)}
			onSelect={(id) => void selectProfile(id)}
			onDelete={(id) => void deleteProfile(id)}
		/>
	</ListSidebar>

	<section class="col-content">
		{#if draft}
			{#key draft.id}
				<ProfileEditor
					{draft}
					requestSave={scheduleSave}
					onManageSecrets={() => void openSecrets()}
				/>
			{/key}
		{:else if selected === SECRETS}
			<SecretsManager />
		{:else}
			<div class="empty-pane">Select a profile, or create one.</div>
		{/if}
	</section>
</div>

<style>
	.connections-view {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
	}

	.secrets-item {
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		margin-bottom: var(--spacing-xs);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.secrets-item:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.secrets-item.selected {
		background-color: var(--color-selected);
		color: var(--color-text-primary);
	}
	.secrets-item.selected:hover {
		background-color: var(--color-hover);
	}
	.secrets-item:focus-visible {
		outline: none;
		box-shadow: 0 0 0 2px var(--color-focus);
	}

	.empty {
		margin: 0;
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.empty-pane {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
</style>
