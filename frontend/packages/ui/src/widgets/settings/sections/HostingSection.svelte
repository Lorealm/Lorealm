<script lang="ts">
	import { configStore, type SaveState } from '$lib/stores/serverConfig.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '$lib/components/NumberInput.svelte';

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

<SettingsSection title="Hosting" {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label="IP address">
		<input type="text" bind:value={draft.ip} />
	</Field>
	<Field label="Port">
		<NumberField
			value={draft.port}
			min={1}
			max={65535}
			onchange={(v) => v !== '' && (draft.port = v)}
		/>
	</Field>
</SettingsSection>
