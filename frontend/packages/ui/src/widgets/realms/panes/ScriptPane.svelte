<script lang="ts">
	import { ScrollText } from '@lucide/svelte';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import { realmDraft } from '../draft.svelte.js';

	let { agentKey }: { agentKey: string } = $props();

	const agent = $derived(realmDraft.agent(agentKey));
</script>

{#if agent}
	<div class="pane">
		<header>
			<ScrollText size={13} />
			<span class="handle">{agent.handle || 'unnamed'}</span>
			{#if agent.name}<span class="name">{agent.name}</span>{/if}
			<span class="spacer"></span>
			<span class="hint">runs on this agent's turn</span>
		</header>

		<div class="editor">
			<WeaverEditor
				doc={agent.script}
				onChange={(next) => (agent.script = next)}
				dialect="template"
				profile="realm_script"
			/>
		</div>
	</div>
{:else}
	<p class="gone">That agent is no longer in the cast.</p>
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
	.handle {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
	}
	.name,
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
