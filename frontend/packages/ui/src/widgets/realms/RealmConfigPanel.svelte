<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { REALM_CONFIG_DEFAULTS, type RealmConfig } from '$lib/types/realm.js';

	type Field = { key: keyof RealmConfig; label: string; tokens: string };

	const FIELDS: Field[] = [
		{ key: 'speech_format', label: 'Speech', tokens: '{name}, {content}' },
		{ key: 'action_format', label: 'Action', tokens: '{name}, {tool}' },
		{ key: 'tool_result_format', label: 'Tool result', tokens: '{tool}, {output}' },
		{ key: 'state_change_format', label: 'State change', tokens: '{scope}, {changes}' },
		{ key: 'state_write_format', label: 'State write', tokens: '{key}, {value}' },
		{ key: 'state_delete_format', label: 'State delete', tokens: '{key}' }
	];

	let open = $state(false);
	let saving = $state(false);

	// Draft kept in sync with the stored config. An empty field means "use the
	// built-in default".
	let draft = $state<Record<string, string>>(toDraft(realmStore.config));

	$effect(() => {
		draft = toDraft(realmStore.config);
	});

	function toDraft(cfg: RealmConfig): Record<string, string> {
		const d: Record<string, string> = {};
		for (const f of FIELDS) d[f.key] = cfg[f.key] ?? '';
		return d;
	}

	function toConfig(): RealmConfig {
		const cfg: RealmConfig = {};
		for (const f of FIELDS) {
			const raw = draft[f.key] ?? '';
			cfg[f.key] = raw.trim() === '' ? null : raw;
		}
		return cfg;
	}

	const dirty = $derived(
		FIELDS.some((f) => (draft[f.key] ?? '') !== (realmStore.config[f.key] ?? ''))
	);

	async function save() {
		if (saving) return;
		saving = true;
		try {
			await realmStore.saveConfig(toConfig());
		} finally {
			saving = false;
		}
	}
</script>

<section class="panel">
	<button type="button" class="header" onclick={() => (open = !open)}>
		<span class="chevron">{open ? '▾' : '▸'}</span>
		<h3>Formatting</h3>
		<span class="sub">how turns read in agent feeds</span>
	</button>

	{#if open}
		<div class="body">
			<p class="note">
				How this realm renders turns in agent feeds. Each field is a template — leave it blank
				to use the built-in default shown as the placeholder.
			</p>

			<form
				class="fields"
				onsubmit={(e) => {
					e.preventDefault();
					save();
				}}
			>
				{#each FIELDS as field (field.key)}
					<label>
						<span class="label">{field.label}<em>{field.tokens}</em></span>
						<input
							type="text"
							bind:value={draft[field.key]}
							placeholder={REALM_CONFIG_DEFAULTS[field.key]}
							spellcheck="false"
						/>
					</label>
				{/each}
				<div class="actions">
					<button type="submit" class="primary" disabled={saving || !dirty}>
						{saving ? 'Saving…' : 'Save'}
					</button>
				</div>
			</form>
		</div>
	{/if}
</section>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		background-color: var(--color-bg-panel);
	}

	.header {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		background: none;
		border: none;
		padding: var(--spacing-md);
		text-align: left;
		width: 100%;
	}

	.header h3 {
		margin: 0;
		font-size: 0.95rem;
	}

	.chevron {
		color: var(--color-text-muted);
		font-size: 0.8rem;
	}

	.sub {
		font-size: 0.8rem;
		color: var(--color-text-muted);
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: 0 var(--spacing-md) var(--spacing-md);
	}

	.note {
		margin: 0;
		font-size: 0.85rem;
		color: var(--color-text-muted);
		line-height: 1.5;
	}

	.fields {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.label {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: var(--spacing-sm);
		font-size: 0.8rem;
		color: var(--color-text-secondary);
	}

	.label em {
		font-style: normal;
		font-family: var(--font-mono, monospace);
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	input {
		font-family: var(--font-mono, monospace);
		font-size: 0.85rem;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}

	button.primary:disabled {
		opacity: 0.5;
	}
</style>
