<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { showError, showToast } from '$lib/stores/errorToast.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { ArrowLeft, Play } from '@lucide/svelte';
	import PickerDialog from './PickerDialog.svelte';
	import RealmEditor from './RealmEditor.svelte';
	import RealmList from './RealmList.svelte';
	import { realmEditor } from './realmEditor.svelte.js';

	const api = useApi();

	const realm = $derived(realmStore.realm);
	let starting = $state(false);

	const scenarios = $derived(realm?.definition.scenarios ?? []);
	/** '' = let the server pick (only/none). Required when several exist. */
	let scenarioPick = $state<string>('');
	$effect(() => {
		if (!scenarios.some((s) => s.id === scenarioPick)) scenarioPick = scenarios[0]?.id ?? '';
	});

	let lastRealmId: string | null = null;
	$effect(() => {
		const id = realm?.id ?? null;
		if (id !== lastRealmId) {
			lastRealmId = id;
			realmEditor.reset();
		}
	});

	async function back() {
		await realmStore.flushAgentSaves();
		navigator.start().realms();
	}

	/** Instantiate the realm and play it */
	async function play() {
		if (!realm || starting) return;
		starting = true;
		try {
			await realmStore.flushAgentSaves();
			const full = await api.chat.realm.start({
				realm_id: realm.id,
				scenario_id: scenarioPick || null
			});
			await chatStore.loadConversation(full.metadata.id);
			navigator.start().write();
			showToast({
				variant: 'info',
				title: 'Playthrough started',
				message: `${realm.definition.name || 'This realm'} is now open in Write.`
			});
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), {
				title: 'Failed to start playthrough',
				durationMs: 5000
			});
		} finally {
			starting = false;
		}
	}
</script>

<div class="shell">
	<header class="topbar">
		<div class="left">
			{#if realm}
				<button type="button" class="icon" title="Back to Realms" onclick={back}>
					<ArrowLeft size={18} />
				</button>
			{/if}
			<div class="title">{realm ? realm.definition.name || 'Untitled Realm' : 'Realms'}</div>
		</div>
		{#if realm}
			<div class="playGroup">
				{#if scenarios.length > 1}
					<select bind:value={scenarioPick} title="Scenario to open with">
						{#each scenarios as s (s.id)}
							<option value={s.id}>{s.name || 'Untitled scenario'}</option>
						{/each}
					</select>
				{/if}
				<button
					type="button"
					class="primary"
					onclick={play}
					disabled={starting || realmStore.agents.length === 0}
					title={realmStore.agents.length === 0
						? 'Add at least one agent first'
						: "Starts a fresh conversation in Write, using this realm's current cast"}
				>
					<Play size={14} />
					{starting ? 'Starting…' : 'Play'}
				</button>
			</div>
		{/if}
	</header>

	<div class="body">
		<div class="main">
			{#if realm}
				<RealmEditor />
			{:else}
				<RealmList />
			{/if}
		</div>
	</div>

	{#if realmEditor.pickerKind}
		<PickerDialog kind={realmEditor.pickerKind} />
	{/if}
</div>

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

	.playGroup {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.playGroup > select {
		height: 32px;
		max-width: 220px;
	}

	.primary {
		display: flex;
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

	.shell :global(input:not([type='checkbox'])),
	.shell :global(select),
	.shell :global(textarea) {
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-medium);
	}

	.shell :global(input:not([type='checkbox']):focus),
	.shell :global(select:focus),
	.shell :global(textarea:focus) {
		border-color: var(--color-border-active);
	}
</style>
