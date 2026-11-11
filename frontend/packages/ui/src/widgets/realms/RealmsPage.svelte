<script lang="ts">
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { ArrowLeft, Play, Save } from '@lucide/svelte';
	import RealmCards from './RealmCards.svelte';
	import RealmEditor from './RealmEditor.svelte';
	import BlueprintList from './BlueprintList.svelte';
	import InstantiateDialog from './InstantiateDialog.svelte';
	import { blueprintStore } from '$lib/stores/blueprint.svelte.js';
	import { realmDraft } from './draft.svelte.js';
	import { panes } from './panes.svelte.js';
	import { toWtn } from './serialize.js';

	type Tab = 'realms' | 'blueprints';
	let tab = $state<Tab>('realms');

	const blueprint = $derived(blueprintStore.current);
	const wtn = $derived(
		toWtn(
			{
				name: realmDraft.name,
				description: realmDraft.description,
				tags: realmDraft.tags,
				director: realmDraft.director,
				scenarios: realmDraft.scenarios,
				config: realmDraft.config,
				modules: realmDraft.modules
			},
			realmDraft.inputs,
			realmDraft.cast
		)
	);

	let building = $state(false);
	let asking = $state(false);

	$effect(() => {
		blueprintStore.queueSave(realmDraft.name, wtn.source, realmDraft.isEmpty);
	});

	async function save() {
		if (!blueprint) return;
		await blueprintStore.save(realmDraft.name, wtn.source, realmDraft.isEmpty);
	}

	/** Build a realm from the active blueprint */
	async function build() {
		if (!blueprint) return;
		if (!(await blueprintStore.save(realmDraft.name, wtn.source, realmDraft.isEmpty))) return;
		if (blueprintStore.inputs.length > 0) {
			asking = true;
			return;
		}
		await run({});
	}

	async function run(inputs: Record<string, unknown>) {
		if (!blueprint) return;
		building = true;
		try {
			const made = await blueprintStore.expand(blueprint.id, inputs);
			if (!made) return;
			asking = false;
			showToast({
				variant: 'info',
				title: 'Realm built',
				message: `${made.definition.name || 'It'} is ready to play.`
			});
			await realmStore.loadList();
		} finally {
			building = false;
		}
	}

	/** Leaving must not drop an edit still sitting on the debounce. */
	async function closeBlueprint() {
		await blueprintStore.flush();
		blueprintStore.close();
		realmDraft.reset();
		panes.reset();
	}
</script>

<div class="shell">
	<header class="topbar">
		<div class="left">
			{#if blueprint}
				<button
					type="button"
					class="icon"
					{@attach tooltip('Back to Realms')}
					onclick={closeBlueprint}
				>
					<ArrowLeft size={18} />
				</button>
				<input
					class="nameField"
					value={realmDraft.name}
					aria-label="Blueprint name"
					oninput={(e) => (realmDraft.name = e.currentTarget.value)}
				/>
			{:else}
				<div class="title">Realms</div>
			{/if}
		</div>

		{#if blueprint}
			<div class="actions">
				{#if blueprintStore.saveStatus === 'error'}
					<span class="saveError" {@attach tooltip(blueprintStore.saveError ?? '')}>
						Unsaved — the server refused it
					</span>
				{:else if blueprintStore.saveStatus === 'saving'}
					<span class="saving">Saving…</span>
				{:else if blueprintStore.saveStatus === 'blocked'}
					<span
						class="saveError"
						{@attach tooltip(
							'The editor is empty but this blueprint is not. Automatic saving stopped so nothing is accidentally lost. Press Save if you meant to clear it.'
						)}
					>
						AutoSave refused: editor is empty
					</span>
				{:else if blueprintStore.saveStatus === 'dirty'}
					<span class="saving">Unsaved changes</span>
				{:else if blueprintStore.saveStatus === 'saved'}
					<span class="saved">Saved</span>
				{/if}
				<button type="button" onclick={save} disabled={blueprintStore.saveStatus === 'saving'}>
					<Save size={13} /> Save
				</button>
				<button type="button" class="primary" onclick={build} disabled={building}>
					<Play size={13} /> Build realm
				</button>
			</div>
		{/if}
	</header>

	<div class="body">
		<div class="main">
			{#if blueprint}
				<RealmEditor />
			{:else}
				<div class="tabs" role="tablist" aria-label="Realms and blueprints">
					<button
						type="button"
						role="tab"
						aria-selected={tab === 'realms'}
						class:active={tab === 'realms'}
						onclick={() => (tab = 'realms')}
					>
						Realms
					</button>
					<button
						type="button"
						role="tab"
						aria-selected={tab === 'blueprints'}
						class:active={tab === 'blueprints'}
						onclick={() => (tab = 'blueprints')}
					>
						Blueprints
					</button>
				</div>
				<div class="lists">
					{#if tab === 'realms'}
						<RealmCards />
					{:else}
						<BlueprintList />
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>

{#if asking}
	<InstantiateDialog
		specs={blueprintStore.inputs}
		busy={building}
		onbuild={run}
		oncancel={() => (asking = false)}
	/>
{/if}

<style>
	.shell {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		color: var(--color-text-primary);
	}

	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-shrink: 0;
		height: var(--height-header);
		padding: 0 var(--spacing-lg);
		background: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.left {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}

	.title {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-h5);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		background: none;
		border: none;
		color: var(--color-text-secondary);
	}

	.icon:hover {
		color: var(--color-text-primary);
	}

	.tabs {
		display: flex;
		flex: 0 0 auto;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tabs button {
		padding: var(--spacing-sm) var(--spacing-md);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-text-muted);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
	}
	.tabs button:hover {
		color: var(--color-text-secondary);
	}
	.tabs button.active {
		border-bottom-color: var(--color-primary-light);
		color: var(--color-text-primary);
	}

	.lists {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
	}
	.nameField {
		min-width: 12em;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h5);
		font-weight: 600;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.actions button:not(.primary) {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-md);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.actions button:not(.primary):hover:not(:disabled) {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.saved {
		color: var(--color-success-text);
		font-size: var(--font-size-xs);
	}
	.saving {
		color: var(--color-warning-text);
		font-size: var(--font-size-xs);
	}
	.saveError {
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
	}

	.primary {
		display: flex;
		padding: var(--spacing-xs) var(--spacing-md);
		border: none;
		border-radius: var(--radius-sm);
		font-size: var(--font-size-sm);
		align-items: center;
		gap: var(--spacing-xs);
		height: 32px;
		white-space: nowrap;
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}

	.primary:hover:not(:disabled) {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}

	.body {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	.main {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		overflow: hidden;
	}

	/* The default skin for form controls anywhere in the realms shell. */
	.shell :global(input:not([type='checkbox']):not(.plain)),
	.shell :global(select:not(.plain)),
	.shell :global(textarea:not(.plain)) {
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-medium);
	}

	.shell :global(input:not([type='checkbox']):not(.plain):focus),
	.shell :global(select:not(.plain):focus),
	.shell :global(textarea:not(.plain):focus) {
		border-color: var(--color-border-active);
	}
</style>
