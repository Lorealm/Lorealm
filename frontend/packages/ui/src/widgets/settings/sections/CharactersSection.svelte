<script lang="ts">
	import { configStore, type SaveState } from '$lib/stores/serverConfig.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '$lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.characters));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.characters));

	function reset() {
		draft = $state.snapshot(configStore.config!.characters);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			await configStore.patch('characters', $state.snapshot(draft));
			draft = $state.snapshot(configStore.config!.characters);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title="Characters" {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label="Upload limit (MB)">
		<NumberField
			value={draft.upload_limit_mb}
			min={0}
			onchange={(v) => v !== '' && (draft.upload_limit_mb = v)}
		/>
	</Field>
	<Field label="Edit session TTL (seconds)">
		<NumberField
			value={draft.edit_session_ttl}
			min={0}
			onchange={(v) => v !== '' && (draft.edit_session_ttl = v)}
		/>
	</Field>
</SettingsSection>
