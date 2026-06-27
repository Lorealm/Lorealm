<script lang="ts">
	import { configStore, type SaveState } from '$lib/stores/serverConfig.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '$lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.users));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.users));

	function reset() {
		draft = $state.snapshot(configStore.config!.users);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			await configStore.patch('users', $state.snapshot(draft));
			draft = $state.snapshot(configStore.config!.users);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title="Users" {dirty} {saveState} onsave={save} onreset={reset}>
	<h3>Password length</h3>
	<div class="pair">
		<Field label="Min">
			<NumberField
				value={draft.password_length.min}
				min={0}
				onchange={(v) => v !== '' && (draft.password_length.min = v)}
			/>
		</Field>
		<Field label="Max">
			<NumberField
				value={draft.password_length.max}
				min={0}
				onchange={(v) => v !== '' && (draft.password_length.max = v)}
			/>
		</Field>
	</div>

	<h3>Username length</h3>
	<div class="pair">
		<Field label="Min">
			<NumberField
				value={draft.username_length.min}
				min={0}
				onchange={(v) => v !== '' && (draft.username_length.min = v)}
			/>
		</Field>
		<Field label="Max">
			<NumberField
				value={draft.username_length.max}
				min={0}
				onchange={(v) => v !== '' && (draft.username_length.max = v)}
			/>
		</Field>
	</div>

	<h3>Registration</h3>
	<label class="check">
		<input type="checkbox" bind:checked={draft.disable_register} />
		<span>Disable registration</span>
	</label>
	<label class="check">
		<input type="checkbox" bind:checked={draft.create_default_resources} />
		<span>Create default resources for new users</span>
	</label>
</SettingsSection>

<style>
	.pair {
		display: flex;
		gap: var(--spacing-lg);
	}
	.pair :global(.field) {
		flex: 1 1 0;
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
