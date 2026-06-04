<script lang="ts">
	import { contextInspector } from '$lib/stores/contextInspector.svelte.js';
	import { parseContextBlocks } from '$lib/types/context.js';
	import { X } from '@lucide/svelte';

	// Re-parse whenever the inspected message changes. `null` => this message has
	// no trace (handled in the body); the inspector should only be opened for
	// messages that do, but we stay defensive.
	let blocks = $derived(
		contextInspector.message ? parseContextBlocks(contextInspector.message.metadata) : null
	);
	let totalChars = $derived(blocks?.reduce((n, b) => n + b.content.length, 0) ?? 0);

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

			<div class="body">
				{#if blocks === null}
					<div class="empty">
						This message carries no context trace. Only turns generated through the weaver pipeline
						record one.
					</div>
				{:else if blocks.length === 0}
					<div class="empty">The pipeline produced no context blocks for this turn.</div>
				{:else}
					{#each blocks as block, i (i)}
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
								<span>node {block.origin.node_id || '—'}</span>
								{#if block.origin.kind.kind === 'source'}
									<span>source {block.origin.kind.source_id || '—'}</span>
								{/if}
								{#if block.origin.path.length > 0}
									<span class="path">{block.origin.path.join(' › ')}</span>
								{/if}
							</div>

							<pre class="content">{block.content}</pre>
						</div>
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

	/* Each block indents by its group depth so nesting reads at a glance, and
	   gets a role-coloured left rail for quick scanning down the stack. */
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

	/* Transparent icon button, same as MessageBubble; global `button:hover`
	   supplies the hover background + accent from standardControls.css. */
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
