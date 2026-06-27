<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { parseContextBlocks, groupAdjacentBlocks } from '$lib/types/context.js';
	import { resolveBlockContent } from '$lib/types/content.js';
	import { ChevronDown } from '@lucide/svelte';

	let latest = $derived.by(() => {
		const path = chatStore.activePath;
		for (let i = path.length - 1; i >= 0; i--) {
			const blocks = parseContextBlocks(path[i].metadata);
			if (blocks) return { id: path[i].id, blocks };
		}
		return null;
	});

	let groups = $derived(latest ? groupAdjacentBlocks(latest.blocks) : null);

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
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-height, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

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

	.row.child {
		padding-left: var(--spacing-xl);
		border-left: 2px solid var(--color-border-subtle);
	}
	.detail.child {
		padding-left: var(--spacing-xl);
		border-left: 2px solid var(--color-border-subtle);
	}

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
