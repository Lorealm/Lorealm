<script lang="ts">
	import { Copy, FileCode2 } from '@lucide/svelte';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import { realmDraft } from '../draft.svelte.js';
	import { toWtn } from '../serialize.js';
	import { inputVariables } from '../variables.js';

	const wtn = $derived(
		toWtn(
			{
				name: realmDraft.name,
				description: realmDraft.description,
				introduction: realmDraft.introduction,
				tags: realmDraft.tags,
				director: realmDraft.director,
				scenarios: realmDraft.scenarios,
				config: realmDraft.config,
				modules: realmDraft.modules
			},
			realmDraft.inputs,
			realmDraft.cast
		)
	);

	async function copy() {
		await navigator.clipboard.writeText(wtn.source);
		showToast({ variant: 'info', message: 'Document copied.' });
	}
</script>

<div class="pane">
	<header>
		<FileCode2 size={13} />
		<span class="title">{wtn.hasInputs ? 'Blueprint' : 'Realm'}</span>
		<span class="spacer"></span>
		<button type="button" onclick={copy}>
			<Copy size={12} /> Copy
		</button>
	</header>

	<div class="editor">
		<WeaverEditor
			doc={wtn.source}
			dialect="document"
			profile="realm_template"
			variables={inputVariables()}
			readOnly
		/>
	</div>
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
	header {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		color: var(--color-text-secondary);
	}
	.title {
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.spacer {
		flex: 1 1 auto;
	}
	header button {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 3px var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	header button:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.editor {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		padding: var(--spacing-sm);
	}
</style>
