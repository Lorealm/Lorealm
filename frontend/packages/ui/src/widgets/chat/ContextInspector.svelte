<script lang="ts">
	import { contextInspector } from '$lib/stores/contextInspector.svelte.js';
	import { parseContextBlocks, groupAdjacentBlocks, type TracedBlock } from '$lib/types/context.js';
	import { resolveBlockContent } from '$lib/types/content.js';
	import { ChevronDown, X } from '@lucide/svelte';
	import { SvelteSet } from 'svelte/reactivity';

	let blocks = $derived(
		contextInspector.message ? parseContextBlocks(contextInspector.message.metadata) : null
	);
	let resolved = $derived(
		blocks?.map((b) => ({ block: b, content: resolveBlockContent(b) })) ?? null
	);
	let totalChars = $derived(resolved?.reduce((n, r) => n + r.content.text.length, 0) ?? 0);

	let groups = $derived(blocks ? groupAdjacentBlocks(blocks) : null);
	let collapsedGroups = new SvelteSet<number>();
	$effect(() => {
		void contextInspector.message?.id;
		collapsedGroups.clear();
	});
	function toggleGroup(gi: number) {
		if (collapsedGroups.has(gi)) collapsedGroups.delete(gi);
		else collapsedGroups.add(gi);
	}

	function roleClass(role: string): 'system' | 'user' | 'assistant' | 'unknown' {
		return role === 'system' || role === 'user' || role === 'assistant' ? role : 'unknown';
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') contextInspector.close();
	}
</script>

<svelte:window onkeydown={onKey} />

{#if contextInspector.message}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) contextInspector.close();
		}}
	>
		<div class="modal" role="dialog" aria-modal="true" aria-label="Context inspector">
			<div class="header">
				<div class="titles">
					<h2>Context Inspector</h2>
					<span class="sub">
						{#if blocks === null}
							no trace on this message
						{:else}
							{blocks.length} block{blocks.length === 1 ? '' : 's'} · ~{totalChars} chars
						{/if}
					</span>
				</div>
				<button class="icon-btn" title="Close" onclick={() => contextInspector.close()}>
					<X />
				</button>
			</div>

			{#snippet blockCard(block: TracedBlock)}
				{@const content = resolveBlockContent(block)}
				<div
					class="block accent-{roleClass(block.role)}"
					style="--depth: {block.origin.path.length}"
				>
					<div class="block-head">
						<span class="role role-{roleClass(block.role)}">{block.role}</span>
						<span class="kind kind-{block.origin.kind.kind}">
							{block.origin.kind.kind.replace('_', ' ')}
						</span>
						<span class="node-title">{block.origin.node_title || '(untitled node)'}</span>
					</div>

					<div class="meta">
						{#if content.label}<span class="ref">{content.label}</span>{/if}
						<span>node {block.origin.node_id || '—'}</span>
						{#if block.origin.kind.kind === 'source'}
							<span>source {block.origin.kind.source_id || '—'}</span>
						{/if}
						{#if block.origin.path.length > 0}
							<span class="path">{block.origin.path.join(' › ')}</span>
						{/if}
					</div>

					{#if content.text}
						<pre class="content">{content.text}</pre>
					{:else}
						<div class="unresolved">content not reconstructed</div>
					{/if}
				</div>
			{/snippet}

			<div class="body">
				{#if groups === null}
					<div class="empty">
						This message carries no context trace. Only turns generated through the weaver pipeline
						record one.
					</div>
				{:else if groups.length === 0}
					<div class="empty">The pipeline produced no context blocks for this turn.</div>
				{:else}
					{#each groups as group, gi (gi)}
						{#if group.blocks.length > 1}
							<button
								class="group-head"
								aria-expanded={!collapsedGroups.has(gi)}
								onclick={() => toggleGroup(gi)}
							>
								<span class="chev" class:open={!collapsedGroups.has(gi)}><ChevronDown /></span>
								<span class="group-title">{group.title || '(untitled node)'}</span>
								<span class="group-count">{group.blocks.length} blocks</span>
							</button>
							{#if !collapsedGroups.has(gi)}
								{#each group.blocks as block, bi (bi)}
									{@render blockCard(block)}
								{/each}
							{/if}
						{:else}
							{@render blockCard(group.blocks[0])}
						{/if}
					{/each}
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xl);
		background: var(--color-overlay-bg);
		backdrop-filter: var(--backdrop-blur-sm);
	}

	.modal {
		display: flex;
		flex-direction: column;
		width: min(760px, 100%);
		max-height: 85vh;
		background: var(--color-bg-modal);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-2xl);
		box-shadow: var(--shadow-xl);
		overflow: hidden;
	}

	.header {
		display: flex;
		flex-direction: row;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.titles {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.header h2 {
		margin: 0;
		color: var(--color-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h3);
	}
	.sub {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		overflow-y: auto;
	}
	.empty {
		color: var(--color-text-muted);
		line-height: var(--line-height-normal);
	}

	.group-head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		cursor: pointer;
	}
	.group-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
		font-weight: var(--font-weight-semibold);
	}
	.group-count {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev.open {
		transform: rotate(180deg);
	}
	.chev :global(svg) {
		width: 16px;
		height: 16px;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		margin-left: calc(var(--depth) * var(--spacing-lg));
		padding: var(--spacing-md);
		background: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-subtle);
		border-left: 3px solid var(--color-border-medium);
		border-radius: var(--radius-md);
	}
	.block.accent-system {
		border-left-color: var(--color-role-system);
	}
	.block.accent-user {
		border-left-color: var(--color-role-user);
	}
	.block.accent-assistant {
		border-left-color: var(--color-role-assistant);
	}

	.block-head {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.node-title {
		color: var(--color-text-primary);
		font-weight: var(--font-weight-semibold);
	}

	.role {
		padding: 2px var(--spacing-sm);
		border-radius: var(--radius-full);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-semibold);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.role-system {
		background: var(--color-role-system-bg);
		color: var(--color-role-system-text);
	}
	.role-user {
		background: var(--color-role-user-bg);
		color: var(--color-role-user-text);
	}
	.role-assistant {
		background: var(--color-role-assistant-bg);
		color: var(--color-role-assistant-text);
	}
	.role-unknown {
		background: var(--color-bg-elevated-3);
		color: var(--color-text-muted);
	}

	.kind {
		padding: 2px var(--spacing-sm);
		border-radius: var(--radius-sm);
		border: 1px solid transparent;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.kind-text {
		background: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
	}
	.kind-source {
		background: var(--color-info-bg);
		color: var(--color-info-text);
		border-color: var(--color-info-border);
	}
	.kind-group_wrapper {
		background: var(--color-warning-bg);
		color: var(--color-warning-text);
		border-color: var(--color-warning-border);
	}

	.meta {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--spacing-sm) var(--spacing-md);
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.path {
		color: var(--color-text-secondary);
		overflow-wrap: anywhere;
	}
	.ref {
		color: var(--color-text-accent);
	}

	.unresolved {
		padding: var(--spacing-sm) var(--spacing-md);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-style: italic;
	}

	.content {
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.icon-btn {
		flex: 0 0 auto;
		background-color: transparent;
		justify-content: center;
		align-items: center;
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.icon-btn :global(svg) {
		width: 18px;
		height: 18px;
	}
</style>
