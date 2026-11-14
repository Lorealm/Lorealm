<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '#lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.plugins));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.plugins));

	function addIndex() {
		draft.indexes.push('');
	}
	function removeIndex(i: number) {
		draft.indexes.splice(i, 1);
	}

	function reset() {
		draft = $state.snapshot(configStore.config!.plugins);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			const payload = $state.snapshot(draft);
			payload.indexes = payload.indexes.map((url) => url.trim()).filter((url) => url !== '');
			await configStore.patch('plugins', payload);
			draft = $state.snapshot(configStore.config!.plugins);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title={t.nav.server.plugins} {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label={t.settings.server.pluginMemoryLimit} hint={t.settings.server.pluginMemoryLimitHint}>
		<NumberField
			value={draft.memory_limit_mb}
			min={1}
			onchange={(v) => v !== '' && (draft.memory_limit_mb = v)}
		/>
	</Field>
	<Field label={t.settings.server.pluginLogCap} hint={t.settings.server.pluginLogCapHint}>
		<NumberField value={draft.log_cap} min={0} onchange={(v) => v !== '' && (draft.log_cap = v)} />
	</Field>
	<Field
		label={t.settings.server.pluginGenerateTimeout}
		hint={t.settings.server.pluginGenerateTimeoutHint}
	>
		<NumberField
			value={draft.generate_timeout}
			min={1}
			onchange={(v) => v !== '' && (draft.generate_timeout = v)}
		/>
	</Field>

	<h3>{t.settings.server.marketplaceIndexes}</h3>
	<p class="note">{t.settings.server.marketplaceIndexesHint}</p>
	{#each draft.indexes, i (i)}
		<div class="row">
			<input type="url" bind:value={draft.indexes[i]} placeholder="https://…/index.json" />
			<button type="button" class="remove" onclick={() => removeIndex(i)}
				>{t.settings.server.remove}</button
			>
		</div>
	{/each}
	<button type="button" class="add" onclick={addIndex}>{t.settings.server.addIndex}</button>
</SettingsSection>

<style>
	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.row {
		display: flex;
		gap: var(--spacing-sm);
		align-items: center;
	}
	.remove,
	.add {
		background: none;
		flex: 0 0 auto;
	}
	.add {
		align-self: flex-start;
	}
</style>
