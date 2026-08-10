<script lang="ts">
	import { onDestroy } from 'svelte';
	import { navigator } from '$lib/navigation/navigator.js';
	import { router } from '$lib/navigation/router.svelte.js';
	import { SECRETS } from '$lib/navigation/routes.js';
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import { PROVIDER_SCHEMAS } from '$lib/types/schema.js';
	import type { Snowflake } from '$lib/types/core.js';
	import ProfileList from './ProfileList.svelte';
	import ProfileEditor from './ProfileEditor.svelte';
	import SecretsManager from './SecretsManager.svelte';
	import SaveIndicator from '$lib/components/SaveIndicator.svelte';

	const SAVE_MS = 500;

	const route = $derived(router.current);
	const selected = $derived(route.view === 'connections' ? (route.id ?? null) : null);
	const draft = $derived(connectionStore.editingProfile);

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
	let newName = $state('');
	let newProvider = $state(PROVIDER_SCHEMAS[0]?.provider ?? '');
	let createBusy = $state(false);
	let createError = $state('');

	function openCreate() {
		creating = true;
		newName = '';
		createError = '';
	}

	async function confirmCreate() {
		const name = newName.trim();
		if (!name) {
			createError = 'Name is required.';
			return;
		}
		createBusy = true;
		createError = '';
		try {
			const id = await connectionStore.createProfile(name, newProvider);
			creating = false;
			if (id) await selectProfile(id);
		} catch (e) {
			createError = e instanceof Error ? e.message : String(e);
		} finally {
			createBusy = false;
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
	<section class="col-list">
		<header class="list-header">
			<div class="header-row">
				<span class="view-name">Connections</span>
			</div>
			<div class="header-row">
				<button class="keys" class:active={selected === SECRETS} type="button" onclick={openSecrets}
					>API keys</button
				>
				<button class="add" class:active={creating} type="button" onclick={openCreate}
					>＋ Profile</button
				>
			</div>
			{#if creating}
				<div class="create-form">
					<!-- svelte-ignore a11y_autofocus -->
					<input
						type="text"
						placeholder="Profile name"
						bind:value={newName}
						autofocus
						onkeydown={(e) => e.key === 'Enter' && confirmCreate()}
					/>
					<select bind:value={newProvider}>
						{#each PROVIDER_SCHEMAS as s (s.provider)}
							<option value={s.provider}>{s.label}</option>
						{/each}
					</select>
					<div class="create-actions">
						<button type="button" disabled={createBusy} onclick={confirmCreate}>
							{createBusy ? 'Creating…' : 'Create'}
						</button>
						<button type="button" onclick={() => (creating = false)}>Cancel</button>
					</div>
					{#if createError}
						<span class="create-error">{createError}</span>
					{/if}
				</div>
			{/if}
			<SaveIndicator state={connectionStore.saveState} />
		</header>

		{#if connectionStore.profiles}
			<ProfileList
				profiles={connectionStore.profiles}
				selectedId={selected === SECRETS ? null : selected}
				onActivate={(id) => void activateProfile(id)}
				onSelect={(id) => void selectProfile(id)}
				onDelete={(id) => void deleteProfile(id)}
			/>
		{/if}
	</section>

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
			<div class="empty">Select a profile, or create one.</div>
		{/if}
	</section>
</div>

<style>
	.connections-view {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
	}
	.col-list {
		flex: 0 0 300px;
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-right: 1px solid var(--color-border-subtle);
	}
	.col-content {
		flex: 1 1 auto;
		width: 100%;
		display: flex;
		min-height: 0;
	}
	.list-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.header-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.view-name {
		flex: 1 1 auto;
		min-width: 0;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.keys,
	.add {
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.keys.active,
	.add.active {
		font-weight: bold;
		color: var(--color-text-primary);
	}
	.create-form {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
	}
	.create-actions {
		display: flex;
		gap: var(--spacing-sm);
	}
	.create-error {
		color: var(--color-text-error, #c0392b);
	}
	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--font-size-sm);
		opacity: 0.6;
	}
</style>
