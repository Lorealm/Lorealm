<script lang="ts">
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import { PROVIDER_SCHEMAS } from '$lib/types/schema.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { SecretMetadata } from '$lib/types/connection.js';

	const secrets = $derived(connectionStore.secrets ?? []);

	let renamingId = $state<Snowflake | null>(null);
	let renameLabel = $state('');

	function startRename(s: SecretMetadata) {
		renamingId = s.id;
		renameLabel = s.label;
	}
	async function commitRename() {
		const id = renamingId;
		if (!id) return;
		renamingId = null;
		try {
			await connectionStore.renameSecret(id, renameLabel.trim() || 'Unnamed key');
		} catch (e) {
			console.error(e);
		}
	}

	async function removeSecret(s: SecretMetadata) {
		try {
			await connectionStore.deleteSecret(s.id);
		} catch (e) {
			console.error(e);
		}
	}

	let newProvider = $state(PROVIDER_SCHEMAS[0]?.provider ?? '');
	let newLabel = $state('');
	let newKey = $state('');
	let creating = $state(false);
	let createError = $state('');

	async function addSecret() {
		if (!newLabel.trim() || !newKey.trim()) {
			createError = 'Label and key are required.';
			return;
		}
		creating = true;
		createError = '';
		try {
			await connectionStore.createSecret({
				provider: newProvider,
				label: newLabel.trim(),
				key: newKey.trim()
			});
			newLabel = '';
			newKey = '';
		} catch (e) {
			createError = e instanceof Error ? e.message : String(e);
		} finally {
			creating = false;
		}
	}
</script>

<div class="secrets-manager">
	<section class="group">
		<h3>API keys</h3>
		<ul class="secret-list">
			{#each secrets as secret (secret.id)}
				<li class="row">
					{#if renamingId === secret.id}
						<!-- svelte-ignore a11y_autofocus -->
						<input
							class="rename"
							type="text"
							bind:value={renameLabel}
							autofocus
							onblur={commitRename}
							onkeydown={(e) => e.key === 'Enter' && commitRename()}
						/>
					{:else}
						<span class="label">{secret.label}</span>
					{/if}
					<span class="provider">{secret.provider}</span>
					<code class="hint">{secret.key_hint}</code>
					<button class="action" type="button" onclick={() => startRename(secret)}>Rename</button>
					<button class="action" type="button" onclick={() => removeSecret(secret)}>Delete</button>
				</li>
			{:else}
				<li class="none">No keys stored.</li>
			{/each}
		</ul>
	</section>

	<section class="group">
		<h3>Add key</h3>
		<label class="field">
			<span>Provider</span>
			<select bind:value={newProvider}>
				{#each PROVIDER_SCHEMAS as s (s.provider)}
					<option value={s.provider}>{s.label}</option>
				{/each}
			</select>
		</label>
		<label class="field">
			<span>Label</span>
			<input type="text" bind:value={newLabel} placeholder="Personal key" />
		</label>
		<label class="field">
			<span>Key</span>
			<input type="password" bind:value={newKey} autocomplete="off" />
		</label>
		<div class="form-row">
			<button type="button" disabled={creating} onclick={addSecret}>
				{creating ? 'Adding…' : 'Add key'}
			</button>
			{#if createError}
				<span class="error">{createError}</span>
			{/if}
		</div>
	</section>
</div>

<style>
	.secrets-manager {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
		padding: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		max-width: 560px;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding-bottom: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.group:last-child {
		border-bottom: none;
	}
	.secret-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) 0;
		font-size: var(--font-size-sm);
	}
	.label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
	}
	.rename {
		flex: 1 1 auto;
		min-width: 0;
		font-size: var(--font-size-sm);
	}
	.provider,
	.hint {
		flex: 0 0 auto;
		opacity: 0.6;
	}
	.action {
		flex: 0 0 auto;
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.none {
		font-size: var(--font-size-sm);
		opacity: 0.6;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
	}
	.form-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.error {
		font-size: var(--font-size-sm);
		color: var(--color-text-error, #c0392b);
	}
</style>
