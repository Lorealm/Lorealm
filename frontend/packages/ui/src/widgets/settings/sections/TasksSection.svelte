<script lang="ts">
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '#lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.tasks));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.tasks));

	function reset() {
		draft = $state.snapshot(configStore.config!.tasks);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			await configStore.patch('tasks', $state.snapshot(draft));
			draft = $state.snapshot(configStore.config!.tasks);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title="Tasks" {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label="Cleanup interval (seconds)">
		<NumberField
			value={draft.cleanup_interval}
			min={0}
			onchange={(v) => v !== '' && (draft.cleanup_interval = v)}
		/>
	</Field>
</SettingsSection>
