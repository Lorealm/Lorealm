<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import WeaverField from '$lib/weaver/editor/WeaverField.svelte';
	import type { AgentFilter, AgentKind, Director } from '$lib/types/realm.js';

	/** The stored director, or the rotation a realm falls back to without one. */
	const director = $derived<Director>(
		realmStore.realm?.definition.director ?? { kind: 'round_robin', over: { match: 'all' } }
	);
	const filter = $derived<AgentFilter>(
		director.kind === 'script' ? { match: 'all' } : director.over
	);
	const agents = $derived(realmStore.agents);

	/** `match` collapsed to a flat select value: `all`, a kind, or `handles`. */
	const drawsFrom = $derived(
		filter.match === 'kind' ? filter.kind : filter.match === 'handles' ? 'handles' : 'all'
	);

	function save(next: Director) {
		void realmStore.updateRealm({ director: next });
	}

	function setKind(kind: Director['kind']) {
		if (kind === director.kind) return;
		save(kind === 'script' ? { kind, source: '' } : { kind, over: filter });
	}

	function setDrawsFrom(value: string) {
		if (director.kind === 'script') return;
		const over: AgentFilter =
			value === 'all'
				? { match: 'all' }
				: value === 'handles'
					? { match: 'handles', handles: agents.map((a) => a.handle) }
					: { match: 'kind', kind: value as AgentKind };
		save({ ...director, over });
	}

	function toggleHandle(handle: string) {
		if (director.kind === 'script' || filter.match !== 'handles') return;
		const handles = filter.handles.includes(handle)
			? filter.handles.filter((h) => h !== handle)
			: [...filter.handles, handle];
		save({ ...director, over: { match: 'handles', handles } });
	}
</script>

<div class="field">
	<div class="label">Director <em>— who acts next</em></div>

	<div class="row">
		<select
			value={director.kind}
			onchange={(e) => setKind(e.currentTarget.value as Director['kind'])}
		>
			<option value="round_robin">Round robin</option>
			<option value="random">Random</option>
			<option value="script">Script</option>
		</select>

		{#if director.kind !== 'script'}
			<select value={drawsFrom} onchange={(e) => setDrawsFrom(e.currentTarget.value)}>
				<option value="all">over the whole cast</option>
				<option value="llm">over LLM agents</option>
				<option value="player">over players</option>
				<option value="script">over script agents</option>
				<option value="handles">over a chosen few</option>
			</select>
		{/if}
	</div>

	{#if director.kind !== 'script' && filter.match === 'handles'}
		<div class="handles">
			{#each agents as agent (agent.handle)}
				<label class="pick">
					<input
						type="checkbox"
						checked={filter.handles.includes(agent.handle)}
						onchange={() => toggleHandle(agent.handle)}
					/>
					<span class="mono">{agent.handle}</span>
				</label>
			{/each}
		</div>
	{/if}

	{#if director.kind === 'script'}
		<WeaverField
			doc={director.source}
			onChange={(v) => save({ kind: 'script', source: v })}
			hint="renders one handle; a template, so it can branch"
		/>
		{#if director.source.trim() === ''}
			<div class="warn">Empty — the realm won't start until this names someone.</div>
		{/if}
		<div class="hint">
			Reads <span class="mono">cast:</span> and <span class="mono">turn:</span>. Rotating the LLMs
			without hardcoding how many there are:
			<span class="mono"
				>@[array.at(items: &lbrace;&lbrace;cast:llm.handles&rbrace;&rbrace;, index: @[math.mod(a:
				&lbrace;&lbrace;turn:index&rbrace;&rbrace;, b:
				&lbrace;&lbrace;cast:llm.count&rbrace;&rbrace;)])]</span
			>
		</div>
	{:else}
		<div class="hint">
			Whoever the director offers first, and whose run condition passes, takes the turn.
		</div>
	{/if}
</div>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.label {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.label em {
		color: var(--color-text-muted);
		font-style: normal;
	}

	.row {
		display: flex;
		gap: var(--spacing-sm);
		flex-wrap: wrap;
	}

	.handles {
		display: flex;
		gap: var(--spacing-md);
		flex-wrap: wrap;
		padding: var(--spacing-sm) 0;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: var(--font-size-sm);
	}

	.hint {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}

	.warn {
		font-size: var(--font-size-sm);
		color: var(--color-warning-text);
	}

	.mono {
		font-family: var(--font-family-mono);
	}
</style>
