<script lang="ts">
	import { untrack } from 'svelte';
	import { templateStore } from '#lib/stores/template.svelte.js';
	import type { ContextNode, NodeContent } from '#lib/types/node.js';
	import type { Snowflake } from '#lib/types/core.js';
	import NodeSettings from './NodeSettings.svelte';
	import FieldShell from '#lib/components/FieldShell.svelte';
	import WeaverArea from '#lib/weaver/editor/WeaverArea.svelte';

	type SaveState = 'idle' | 'saving' | 'saved' | 'error';

	let {
		node,
		saveState = $bindable<SaveState>('idle')
	}: {
		node: ContextNode | null;
		saveState?: SaveState;
	} = $props();

	// Local buffer for the text body
	let text = $state('');
	let lastId: Snowflake | null = null;
	let syncedId = $state<Snowflake | null>(null);
	let timer: ReturnType<typeof setTimeout> | null = null;
	let pending: { id: Snowflake; content: NodeContent } | null = null;

	$effect(() => {
		const n = node;
		const nid = n?.id ?? null;
		if (nid === lastId) return;
		untrack(() => {
			void flush();
			lastId = nid;
			saveState = 'idle';
			text = n && n.content.type === 'text' ? n.content.content : '';
			syncedId = nid;
		});
	});

	function onInput(next: string) {
		text = next;
		const n = node;
		if (!n || n.content.type !== 'text') return;
		pending = { id: n.id, content: { type: 'text', content: text, role: n.content.role } };
		saveState = 'saving';
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => void flush(), 600);
	}

	async function flush() {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		const p = pending;
		if (!p) return;
		pending = null;
		try {
			await templateStore.updateNode(p.id, { content: p.content });
			saveState = 'saved';
		} catch {
			saveState = 'error';
		}
	}
</script>

<div class="node-editor">
	{#if !node}
		<div class="empty">Select a node to edit</div>
	{:else}
		<div class="settings-region" class:grow={node.content.type !== 'text'}>
			<NodeSettings {node} bind:saveState />
		</div>

		{#if node.content.type === 'text'}
			<div class="content-region">
				<FieldShell label="Content" tokenText={text} fill>
					{#snippet editor(fill: boolean)}
						{#key syncedId}
							<WeaverArea doc={text} onChange={onInput} hint="Empty prompt" fill autofocus={fill} />
						{/key}
					{/snippet}
				</FieldShell>
			</div>
		{/if}
	{/if}
</div>

<style>
	.node-editor {
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}

	.settings-region {
		flex: 0 0 auto;
		overflow-y: auto;
		padding: var(--spacing-lg);
	}
	/* groups/source nodes have no content body, so the form takes the space */
	.settings-region.grow {
		flex: 1 1 auto;
		min-height: 0;
	}
	/* text nodes: the form is capped so the content editor dominates */
	.settings-region:not(.grow) {
		max-height: 45%;
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.content-region {
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-lg);
	}
</style>
