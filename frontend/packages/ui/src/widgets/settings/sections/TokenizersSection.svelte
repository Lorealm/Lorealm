<script lang="ts">
	import { configStore, type SaveState } from '$lib/stores/serverConfig.svelte';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '$lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.tokenizers));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.tokenizers));

	function addTokenizer() {
		draft.default_tokenizers.push('');
	}
	function removeTokenizer(i: number) {
		draft.default_tokenizers.splice(i, 1);
	}

	function reset() {
		draft = $state.snapshot(configStore.config!.tokenizers);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			const payload = $state.snapshot(draft);
			payload.default_tokenizers = payload.default_tokenizers.filter((t) => t.trim() !== '');
			await configStore.patch('tokenizers', payload);
			draft = $state.snapshot(configStore.config!.tokenizers);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title="Tokenizers" {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label="Max tokenizer size (MB)">
		<NumberField
			value={draft.max_tokenizer_size_mb}
			min={0}
			onchange={(v) => v !== '' && (draft.max_tokenizer_size_mb = v)}
		/>
	</Field>
	<Field label="Tokenizer TTL (seconds)">
		<NumberField
			value={draft.tokenizer_ttl}
			min={0}
			onchange={(v) => v !== '' && (draft.tokenizer_ttl = v)}
		/>
	</Field>

	<h3>Default tokenizers</h3>
	{#each draft.default_tokenizers as _, i (i)}
		<div class="row">
			<input type="text" bind:value={draft.default_tokenizers[i]} placeholder="hf:org/model" />
			<button type="button" class="remove" onclick={() => removeTokenizer(i)}>Remove</button>
		</div>
	{/each}
	<button type="button" class="add" onclick={addTokenizer}>+ Add tokenizer</button>
</SettingsSection>

<style>
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
