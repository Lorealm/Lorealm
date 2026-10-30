<script lang="ts">
	import { onMount } from 'svelte';
	import { useApi } from '$lib/api/client.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import type { InputSpec } from '$lib/types/blueprint.js';
	import InputControl, { blankFor } from '../../realms/InputControl.svelte';
	import { ensureLoaded } from '../../realms/resources.svelte.js';

	let { handle }: { handle?: string } = $props();

	const api = useApi();

	const plugins = $derived(pluginsStore.plugins.filter((p) => p.has_settings));

	let inputs = $state<InputSpec[]>([]);
	let values = $state<Record<string, unknown>>({});
	let saving = $state(false);
	let saved = $state(false);
	let error = $state('');

	onMount(() => {
		void pluginsStore.load();
		// Key settings pick from these.
		void connectionStore.loadSecrets();
	});

	$effect(() => {
		inputs = [];
		saved = false;
		error = '';
		if (handle === undefined) return;
		api.plugins
			.settings(handle)
			.then((settings) => {
				inputs = settings.inputs;
				// Settings like `Ref<ConnectionProfile>` pick from these.
				ensureLoaded(
					settings.inputs.flatMap((spec) =>
						spec.type === 'reference' && spec.kind ? [spec.kind] : []
					)
				);
				values = Object.fromEntries(
					// `null` is a value: an optional key with none picked.
					settings.inputs.map((spec) => [
						spec.name,
						spec.name in settings.values ? settings.values[spec.name] : blankFor(spec)
					])
				);
			})
			.catch((e) => (error = e instanceof Error ? e.message : String(e)));
	});

	async function save() {
		if (handle === undefined) return;
		saving = true;
		saved = false;
		error = '';
		try {
			await api.plugins.saveSettings(handle, values);
			saved = true;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
		saving = false;
	}
</script>

<div class="section">
	<nav class="list">
		{#each plugins as plugin (plugin.handle)}
			<button
				type="button"
				class="item"
				class:active={plugin.handle === handle}
				onclick={() => navigator.start().settings().user().plugins(plugin.handle)}
			>
				{plugin.name}
				{#if !plugin.enabled}<span class="muted">disabled</span>{/if}
			</button>
		{:else}
			<p class="muted">{pluginsStore.loaded ? 'No installed plugin has settings.' : 'Loading…'}</p>
		{/each}
	</nav>

	<div class="form">
		{#if handle === undefined}
			<p class="muted">Pick a plugin to change its settings.</p>
		{:else}
			{#each inputs as spec (spec.name)}
				<div class="field">
					<span class="label">{spec.name}</span>
					<InputControl
						ty={spec}
						value={values[spec.name]}
						ariaLabel={spec.name}
						onchange={(next) => {
							// No pick is `null`, which an optional `Ref` takes.
							values[spec.name] = spec.type === 'reference' && next === '' ? null : next;
							saved = false;
						}}
					/>
				</div>
			{/each}
			{#if inputs.length}
				<div class="actions">
					<button type="button" class="primary" onclick={save} disabled={saving}>
						{saving ? 'Saving…' : 'Save'}
					</button>
					{#if saved}<span class="muted">Saved</span>{/if}
				</div>
			{/if}
			{#if error}<p class="error">{error}</p>{/if}
		{/if}
	</div>
</div>

<style>
	.section {
		display: grid;
		grid-template-columns: minmax(180px, 240px) 1fr;
		gap: var(--spacing-lg);
		flex: 1;
		min-height: 0;
		padding: var(--spacing-sm) 0;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		overflow-y: auto;
	}
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		padding: 6px 14px;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.item.active {
		border-color: currentColor;
	}

	.form {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		max-width: 480px;
		overflow-y: auto;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.label {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.muted {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.error {
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
</style>
