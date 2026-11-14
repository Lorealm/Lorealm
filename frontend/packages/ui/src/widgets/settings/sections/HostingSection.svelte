<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '#lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.hosting));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.hosting));

	function reset() {
		draft = $state.snapshot(configStore.config!.hosting);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			await configStore.patch('hosting', $state.snapshot(draft));
			draft = $state.snapshot(configStore.config!.hosting);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title={t.nav.server.hosting} {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label={t.settings.server.ipAddress}>
		<input type="text" bind:value={draft.ip} />
	</Field>
	<Field label={t.settings.server.port}>
		<NumberField
			value={draft.port}
			min={1}
			max={65535}
			onchange={(v) => v !== '' && (draft.port = v)}
		/>
	</Field>
</SettingsSection>
