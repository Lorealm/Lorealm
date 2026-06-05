<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { parseContextBlocks, groupAdjacentBlocks } from '$lib/types/context.js';
	import { resolveBlockContent } from '$lib/types/content.js';
	import { ChevronDown } from '@lucide/svelte';

	// Track the latest *traced* turn, not literally the last message: user turns
	// and the in-flight streaming bubble carry no trace. Walk the active path back
	// from the leaf and take the first message that has one. Reading `activePath`
	// subscribes us to the tree + stream, so this re-derives as turns complete.
	let latest = $derived.by(() => {
		const path = chatStore.activePath;
		for (let i = path.length - 1; i >= 0; i--) {
			const blocks = parseContextBlocks(path[i].metadata);
			if (blocks) return { id: path[i].id, blocks };
		}
		return null;
	});

	// Group adjacent blocks from the same source node so long runs (chat history
	// especially) fold into one entry instead of stacking. Recomputed per turn.
	let groups = $derived(latest ? groupAdjacentBlocks(latest.blocks) : null);

	// Two-level accordion: one group open at a time, and within it one block's
	// content open. Single-block groups skip the middle step — opening the group
	// shows its content directly. Reset both whenever the tracked turn changes.
	let openGroup = $state<number | null>(null);
	let openBlock = $state<number | null>(null);
	$effect(() => {
		void latest?.id;
		openGroup = null;
		openBlock = null;
	});

	function toggleGroup(gi: number, single: boolean) {
		if (openGroup === gi) {
			openGroup = null;
			openBlock = null;
		} else {
			openGroup = gi;
			openBlock = single ? 0 : null;
		}
	}
	function toggleBlock(bi: number) {
		openBlock = openBlock === bi ? null : bi;
	}

	// Collapse the whole widget down to just its head.
	let collapsed = $state(false);

	function roleClass(role: string): 'system' | 'user' | 'assistant' | 'unknown' {
		return role === 'system' || role === 'user' || role === 'assistant' ? role : 'unknown';
	}
	function abbrevKind(k: string): string {
		return k === 'source' ? 'SRC' : k === 'group_wrapper' ? 'GRP' : 'TXT';
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">Live Context</span>
		{#if latest}
			<span class="count">{latest.blocks.length} block{latest.blocks.length === 1 ? '' : 's'}</span>
		{/if}
	</button>

	{#if !collapsed}
		<div class="list">
			{#if !groups}
				<div class="empty">No traced turn yet.</div>
			{:else if groups.length === 0}
				<div class="empty">Latest turn produced no blocks.</div>
			{:else}
				{#each groups as group, gi (gi)}
					{@const single = group.blocks.length === 1}
					<button
						class="row"
						aria-expanded={openGroup === gi}
						onclick={() => toggleGroup(gi, single)}
					>
						{#if single}
							<span class="dot role-{roleClass(group.blocks[0].role)}"></span>
							<span class="kind kind-{group.blocks[0].origin.kind.kind}"
								>{abbrevKind(group.blocks[0].origin.kind.kind)}</span
							>
						{:else}
							<span class="badge">{group.blocks.length}</span>
						{/if}
						<span class="title">{group.title || '(untitled)'}</span>
						<span class="chev" class:open={openGroup === gi}><ChevronDown /></span>
					</button>

					{#if openGroup === gi}
						{#if single}
							{@const r = resolveBlockContent(group.blocks[0])}
							<div class="detail">
								<div class="meta">
									{#if r.label}<span class="ref">{r.label}</span>{/if}
									<span>node {group.blocks[0].origin.node_id || '—'}</span>
									{#if group.blocks[0].origin.kind.kind === 'source'}
										<span>source {group.blocks[0].origin.kind.source_id || '—'}</span>
									{/if}
								</div>
								{#if r.text}
									<pre class="content">{r.text}</pre>
								{:else}
									<div class="unresolved">content not reconstructed</div>
								{/if}
							</div>
						{:else}
							{#each group.blocks as block, bi (bi)}
								{@const r = resolveBlockContent(block)}
								<button
									class="row child"
									aria-expanded={openBlock === bi}
									onclick={() => toggleBlock(bi)}
								>
									<span class="dot role-{roleClass(block.role)}"></span>
									<span class="title muted">{r.text.trim() || block.role}</span>
									<span class="chev" class:open={openBlock === bi}><ChevronDown /></span>
								</button>
								{#if openBlock === bi}
									<div class="detail child">
										<div class="meta">
											{#if r.label}<span class="ref">{r.label}</span>{/if}
											<span>{block.role}</span>
										</div>
										{#if r.text}
											<pre class="content">{r.text}</pre>
										{:else}
											<div class="unresolved">content not reconstructed</div>
										{/if}
									</div>
								{/if}
							{/each}
						{/if}
					{/if}
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	/* Fixed height so sibling widgets can stack beneath; full width of whatever
	   the sidebar happens to be. Height is a knob, not a hardcode. */
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-height, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	/* Collapsed: only the head remains, so let the box shrink to it and drop the
	   head's own divider (the widget border already separates it from the next). */
	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	/* Head doubles as the collapse toggle. As a button it picks up the global
	   `button:hover` wash; strip the elevated bg/radius and lay it out flat. */
	.head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		text-align: left;
		cursor: pointer;
	}
	.label {
		flex: 1;
		min-width: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.count {
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.list {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
	}
	.empty {
		margin: auto;
		padding: var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-align: center;
	}

	/* Row is a button so the global `button:hover` gives it the hover wash; we
	   strip the elevated bg/radius the base rule applies and lay it out flat. */
	.row {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		cursor: pointer;
	}
	/* Child rows sit under an expanded group header; indent + faint rail to show
	   they belong to the group above. */
	.row.child {
		padding-left: var(--spacing-xl);
		border-left: 2px solid var(--color-border-subtle);
	}
	.detail.child {
		padding-left: var(--spacing-xl);
		border-left: 2px solid var(--color-border-subtle);
	}

	/* Count pill standing in for the kind tag on multi-block group headers. */
	.badge {
		flex: 0 0 auto;
		min-width: 18px;
		padding: 0 var(--spacing-xs);
		text-align: center;
		border-radius: var(--radius-full);
		background: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.dot {
		flex: 0 0 auto;
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background-color: var(--color-text-muted);
	}
	.dot.role-system {
		background-color: var(--color-role-system);
	}
	.dot.role-user {
		background-color: var(--color-role-user);
	}
	.dot.role-assistant {
		background-color: var(--color-role-assistant);
	}

	.kind {
		flex: 0 0 auto;
		padding: 1px var(--spacing-xs);
		border-radius: var(--radius-sm);
		border: 1px solid transparent;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-wide);
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

	/* The flexible cell: takes the slack and truncates, so changing the sidebar
	   width only changes how much title is visible. */
	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
	}
	.title.muted {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
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
		width: 14px;
		height: 14px;
	}

	.detail {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: 0 var(--spacing-md) var(--spacing-sm);
	}
	.meta {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--spacing-xs) var(--spacing-md);
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
		padding: var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-style: italic;
	}
	.content {
		margin: 0;
		max-height: 160px;
		overflow-y: auto;
		padding: var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
