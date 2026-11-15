<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
	import { messageStateStore } from '#lib/stores/messageState.svelte.js';
	import { flattenPaths } from '#lib/types/state.js';
	import { ChevronDown } from '@lucide/svelte';

	let messageState = $derived(messageStateStore.current);
	let loading = $derived(messageStateStore.loading);
	let targetId = $derived(messageStateStore.targetId);

	type Tab = 'activation' | 'persistent';
	let tab = $state<Tab>('activation');
	let collapsed = $state(false);

	let openGroup = $state<string | null>(null);
	let openPool = $state<string | null>(null);

	$effect(() => {
		void targetId;
		openGroup = null;
		openPool = null;
	});

	let activationGroups = $derived.by(() => {
		if (!messageState) return [];
		return Object.entries(messageState.activation).map(([id, act]) => {
			const ids = new Set([
				...Object.keys(act.last_activated),
				...Object.keys(act.sticky_remaining)
			]);
			const entries = [...ids]
				.map((entryId) => ({
					entryId,
					lastActivated: act.last_activated[entryId],
					sticky: act.sticky_remaining[entryId]
				}))
				.sort((a, b) => a.entryId.localeCompare(b.entryId));
			return { id, currentTurn: act.current_turn, entries };
		});
	});

	let persistentPools = $derived.by(() => {
		if (!messageState) return [];
		return Object.entries(messageState.persistent).map(([name, pool]) => ({
			name,
			entries: flattenPaths(pool)
		}));
	});

	let currentTurn = $derived.by(() =>
		activationGroups.length ? Math.max(...activationGroups.map((g) => g.currentTurn)) : null
	);

	function toggleGroup(id: string) {
		openGroup = openGroup === id ? null : id;
	}
	function togglePool(name: string) {
		openPool = openPool === name ? null : name;
	}

	function fmt(value: unknown): string {
		if (typeof value === 'string') return value;
		if (value === null || value === undefined) return String(value);
		if (typeof value === 'object') {
			try {
				return JSON.stringify(value);
			} catch {
				return String(value);
			}
		}
		return String(value);
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">{t.inspector.state.title}</span>
		{#if messageState && currentTurn !== null}
			<span class="count">{t.inspector.state.turn({ n: currentTurn })}</span>
		{/if}
	</button>

	{#if !collapsed}
		{#if !targetId}
			<div class="empty">{t.inspector.state.noMessage}</div>
		{:else if loading && !messageState}
			<div class="empty">{t.chat.inspector.loadingState}</div>
		{:else if !messageState}
			<div class="empty">{t.inspector.state.unavailable}</div>
		{:else}
			<div class="tabs" role="tablist">
				<button
					class="tab"
					class:active={tab === 'activation'}
					role="tab"
					aria-selected={tab === 'activation'}
					onclick={() => (tab = 'activation')}
				>
					{t.chat.inspector.activation}
					<span class="tab-count">{activationGroups.length}</span>
				</button>
				<button
					class="tab"
					class:active={tab === 'persistent'}
					role="tab"
					aria-selected={tab === 'persistent'}
					onclick={() => (tab = 'persistent')}
				>
					{t.chat.inspector.persistent}
					<span class="tab-count">{persistentPools.length}</span>
				</button>
			</div>

			<div class="list">
				{#if tab === 'activation'}
					{#if activationGroups.length === 0}
						<div class="empty">{t.inspector.state.noActivation}</div>
					{:else}
						{#each activationGroups as group (group.id)}
							<button
								class="row"
								aria-expanded={openGroup === group.id}
								onclick={() => toggleGroup(group.id)}
							>
								<span class="badge">T{group.currentTurn}</span>
								{#await lorebookStore.getLorebook(group.id)}
									<span class="title mono">{t.common.loading}</span>
								{:then lorebook}
									<span class="title mono">{lorebook?.name}</span>
								{:catch}
									<span class="title mono">{t.inspector.state.unknown}</span>
								{/await}
								<span class="count">{group.entries.length}</span>
								<span class="chev" class:open={openGroup === group.id}><ChevronDown /></span>
							</button>

							{#if openGroup === group.id}
								<div class="detail">
									{#if group.entries.length === 0}
										<div class="unresolved">{t.chat.inspector.noTrackedEntries}</div>
									{:else}
										{#each group.entries as e (e.entryId)}
											<div class="entry">
												<span class="entry-name" {@attach tooltip(e.entryId)}>{e.entryId}</span>
												<span
													class="chip"
													class:dim={e.lastActivated === undefined}
													{@attach tooltip(
														t.chat.inspector.lastActivated({
															turns: String(e.lastActivated ?? '—')
														})
													)}>{t.chat.inspector.act({ turns: String(e.lastActivated ?? '—') })}</span
												>
												<span
													class="chip"
													class:dim={e.sticky === undefined}
													{@attach tooltip(
														t.chat.inspector.stickyRemaining({ turns: e.sticky ?? 0 })
													)}>{t.chat.inspector.stk({ turns: String(e.sticky ?? '—') })}</span
												>
											</div>
										{/each}
									{/if}
								</div>
							{/if}
						{/each}
					{/if}
				{:else if persistentPools.length === 0}
					<div class="empty">{t.inspector.state.noPersistent}</div>
				{:else}
					{#each persistentPools as pool (pool.name)}
						<button
							class="row"
							aria-expanded={openPool === pool.name}
							onclick={() => togglePool(pool.name)}
						>
							<span class="kind kind-source">{t.chat.inspector.pool}</span>
							<span class="title">{pool.name}</span>
							<span class="count">{pool.entries.length}</span>
							<span class="chev" class:open={openPool === pool.name}><ChevronDown /></span>
						</button>

						{#if openPool === pool.name}
							<div class="detail">
								{#if pool.entries.length === 0}
									<div class="unresolved">{t.chat.inspector.emptyPool}</div>
								{:else}
									{#each pool.entries as kv (kv.key)}
										<div class="kv">
											<span class="kv-key" {@attach tooltip(kv.key)}>{kv.key}</span>
											<pre class="kv-val">{fmt(kv.value)}</pre>
										</div>
									{/each}
								{/if}
							</div>
						{/if}
					{/each}
				{/if}
			</div>
		{/if}
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
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.tabs {
		display: flex;
		flex: 0 0 auto;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tab {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		flex: 1;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		cursor: pointer;
		border-bottom: 2px solid transparent;
	}
	.tab.active {
		color: var(--color-text-primary);
		border-bottom-color: var(--color-text-accent);
	}
	.tab-count {
		font-family: var(--font-family-mono);
		color: var(--color-text-muted);
		letter-spacing: 0;
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
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		cursor: pointer;
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

	.kind {
		flex: 0 0 auto;
		padding: 1px var(--spacing-xs);
		border-radius: var(--radius-sm);
		border: 1px solid transparent;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-wide);
	}
	.kind-source {
		background: var(--color-info-bg);
		color: var(--color-info-text);
		border-color: var(--color-info-border);
	}

	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
	}
	.title.mono {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-secondary);
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
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-md) var(--spacing-sm);
	}

	.entry {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) 0 var(--spacing-xs) var(--spacing-md);
		border-left: 2px solid var(--color-border-subtle);
	}
	.entry-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.chip {
		flex: 0 0 auto;
		padding: 1px var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.chip.dim {
		color: var(--color-text-muted);
		opacity: 0.6;
	}

	.kv {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding-left: var(--spacing-md);
		border-left: 2px solid var(--color-border-subtle);
	}
	.kv-key {
		color: var(--color-text-accent);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.kv-val {
		margin: 0;
		max-height: 120px;
		overflow-y: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.unresolved {
		padding: var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-style: italic;
	}
</style>
