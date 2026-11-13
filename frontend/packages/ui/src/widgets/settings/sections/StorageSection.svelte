<script lang="ts">
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import type { BackendConfig } from '#lib/types/config.js';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '#lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.storage));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.storage));

	function setBackendType(i: number, type: BackendConfig['type']) {
		draft.backends[i] =
			type === 'postgres'
				? { type: 'postgres', connection_string: '', max_connections: 10 }
				: { type: 'memory', max_size: 1000 };
	}
	function addBackend() {
		draft.backends.push({ type: 'postgres', connection_string: '', max_connections: 10 });
	}
	function removeBackend(i: number) {
		draft.backends.splice(i, 1);
	}

	function toggleCache(on: boolean) {
		draft.cache = on
			? { redis_url: 'redis://localhost:6379', ttl_seconds: 300, key_prefix: 'lorealm' }
			: null;
	}

	function reset() {
		draft = $state.snapshot(configStore.config!.storage);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			// Normalize empty postgres connection strings to null
			const payload = $state.snapshot(draft);
			for (const b of payload.backends) {
				if (b.type === 'postgres' && !b.connection_string) b.connection_string = null;
			}
			await configStore.patch('storage', payload);
			draft = $state.snapshot(configStore.config!.storage);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title="Storage" {dirty} {saveState} onsave={save} onreset={reset}>
	<h3>Backends</h3>
	{#each draft.backends as backend, i (i)}
		<div class="backend">
			<div class="backend-head">
				<select
					value={backend.type}
					onchange={(e) => setBackendType(i, e.currentTarget.value as BackendConfig['type'])}
				>
					<option value="postgres">Postgres</option>
					<option value="memory">Memory</option>
				</select>
				<button type="button" class="remove" onclick={() => removeBackend(i)}>Remove</button>
			</div>

			{#if backend.type === 'postgres'}
				<Field label="Connection string" hint="Leave blank to use the server default.">
					<input type="text" bind:value={backend.connection_string} placeholder="postgresql://…" />
				</Field>
				<Field label="Max connections">
					<NumberField
						value={backend.max_connections}
						min={1}
						onchange={(v) => v !== '' && (backend.max_connections = v)}
					/>
				</Field>
			{:else}
				<Field label="Max size">
					<NumberField
						value={backend.max_size}
						min={0}
						onchange={(v) => v !== '' && (backend.max_size = v)}
					/>
				</Field>
			{/if}
		</div>
	{/each}
	<button type="button" class="add" onclick={addBackend}>+ Add backend</button>

	<h3>General</h3>
	<Field label="Data directory">
		<input type="text" bind:value={draft.data_directory} />
	</Field>
	<label class="check">
		<input type="checkbox" bind:checked={draft.require_postgres_fallback} />
		<span>Require Postgres fallback</span>
	</label>

	<h3>Cache</h3>
	<label class="check">
		<input
			type="checkbox"
			class="switch"
			checked={draft.cache != null}
			onchange={(e) => toggleCache(e.currentTarget.checked)}
		/>
		<span>Enable Redis cache</span>
	</label>
	{#if draft.cache != null}
		<Field label="Redis URL">
			<input type="text" bind:value={draft.cache.redis_url} />
		</Field>
		<Field label="TTL (seconds)">
			<NumberField
				value={draft.cache.ttl_seconds}
				min={0}
				onchange={(v) => v !== '' && draft.cache && (draft.cache.ttl_seconds = v)}
			/>
		</Field>
		<Field label="Key prefix">
			<input type="text" bind:value={draft.cache.key_prefix} />
		</Field>
	{/if}
</SettingsSection>

<style>
	.backend {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}
	.backend-head {
		display: flex;
		gap: var(--spacing-sm);
		align-items: center;
	}
	.backend-head select {
		width: auto;
		flex: 1 1 auto;
	}
	.remove,
	.add {
		background: none;
		flex: 0 0 auto;
	}
	.add {
		align-self: flex-start;
	}
	.check {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		cursor: pointer;
	}
	.check span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
</style>
