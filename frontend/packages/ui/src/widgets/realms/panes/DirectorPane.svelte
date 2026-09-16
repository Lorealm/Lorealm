<script lang="ts">
	import { Shuffle } from '@lucide/svelte';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import { realmDraft } from '../draft.svelte.js';

	const director = $derived(realmDraft.director);
</script>

{#if director?.kind === 'script'}
	{@const script = director}
	<div class="pane">
		<header>
			<Shuffle size={13} />
			<span class="title">Director</span>
			<span class="spacer"></span>
			<span class="hint">answers with the handle of whoever acts next</span>
		</header>

		<div class="editor">
			<WeaverEditor
				doc={script.source}
				onChange={(next) => (realmDraft.director = { kind: 'script', source: next })}
				dialect="template"
				profile="realm_director"
			/>
		</div>
	</div>
{:else}
	<p class="gone">This realm no longer picks its turns with a script.</p>
{/if}

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
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.spacer {
		flex: 1 1 auto;
	}
	.editor {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		padding: var(--spacing-sm);
	}
	.gone {
		margin: var(--spacing-lg) auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
