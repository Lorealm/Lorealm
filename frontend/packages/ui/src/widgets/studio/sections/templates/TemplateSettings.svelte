<script lang="ts">
	import { onDestroy } from 'svelte';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import { extractError } from '$lib/types/error.js';
	import type { SaveState } from '$lib/stores/lorebook.svelte.js';
	import type { ContextTemplate } from '$lib/types/template.js';
	import SaveIndicator from '$lib/components/SaveIndicator.svelte';

	const SAVE_MS = 500;

	let { template }: { template: ContextTemplate } = $props();

	// svelte-ignore state_referenced_locally
	let name = $state(template.name);
	// svelte-ignore state_referenced_locally
	let description = $state(template.description);
	let saveState = $state<SaveState>({ status: 'idle' });

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(save, SAVE_MS);
	}
	async function save() {
		saveState = { status: 'saving' };
		try {
			await templateStore.updateTemplate(template.id, { name, description });
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', ...extractError(e) };
		}
	}
	onDestroy(() => clearTimeout(timer));
</script>

<div class="settings-editor">
	<header class="head">
		<h2>Template settings</h2>
		<SaveIndicator state={saveState} />
	</header>

	<label class="field">
		<span>Name</span>
		<input type="text" bind:value={name} oninput={requestSave} />
	</label>

	<label class="field">
		<span>Description</span>
		<textarea rows="3" bind:value={description} oninput={requestSave}></textarea>
	</label>
</div>

<style>
	.settings-editor {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		min-width: 0;
		overflow-y: auto;
		flex: 1 1 auto;
	}
	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.head h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		color: var(--color-text-primary);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.field > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.field input,
	.field textarea {
		width: 100%;
	}
</style>
