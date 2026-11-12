<script lang="ts">
	import { Maximize2, Shuffle, X } from '@lucide/svelte';
	import type { AgentFilter, AgentKind } from '$lib/types/realm.js';
	import { realmDraft } from '../draft.svelte.js';
	import { panes } from '../panes.svelte.js';
	import RadioField from '../RadioField.svelte';

	const AGENT_KINDS: AgentKind[] = ['llm', 'player', 'script'];

	const kind = $derived(realmDraft.director?.kind ?? 'round_robin');
	const filterOf = $derived<AgentFilter>(
		realmDraft.director && realmDraft.director.kind !== 'script'
			? realmDraft.director.over
			: { match: 'all' }
	);

	function setKind(next: string) {
		if (next === 'script') {
			realmDraft.director = { kind: 'script', source: '' };
		} else if (next === 'random') {
			realmDraft.director = { kind: 'random', over: filterOf };
		} else {
			realmDraft.director =
				filterOf.match === 'all' ? null : { kind: 'round_robin', over: filterOf };
		}
	}

	function setFilter(next: AgentFilter) {
		const current = realmDraft.director;
		if (current?.kind === 'script') return;
		if (next.match === 'all' && kind === 'round_robin') {
			realmDraft.director = null;
			return;
		}
		realmDraft.director = { kind: kind === 'random' ? 'random' : 'round_robin', over: next };
	}

	let tagDraft = $state('');

	function addTag() {
		const tag = tagDraft.trim();
		if (tag && !realmDraft.tags.includes(tag)) realmDraft.tags.push(tag);
		tagDraft = '';
	}

	const scriptPreview = $derived(
		realmDraft.director?.kind === 'script'
			? (realmDraft.director.source.split('\n').find((line) => line.trim()) ?? '')
			: ''
	);
</script>

<div class="pane">
	<header>
		<h2>Overview</h2>
	</header>

	<div class="body">
		<label>
			<span>Name</span>
			<input
				value={realmDraft.name}
				placeholder="Untitled"
				oninput={(e) => (realmDraft.name = e.currentTarget.value)}
			/>
		</label>

		<label>
			<span>Description</span>
			<textarea
				rows="4"
				value={realmDraft.description}
				placeholder="What happens here, and who it is for."
				oninput={(e) => (realmDraft.description = e.currentTarget.value)}
			></textarea>
		</label>

		<label>
			<span>Introduction</span>
			<textarea
				rows="8"
				value={realmDraft.introduction}
				placeholder="Shown first to anyone building a realm from this. Say what it is and how to set it up. Markdown works."
				oninput={(e) => (realmDraft.introduction = e.currentTarget.value)}
			></textarea>
		</label>

		<div class="field">
			<span>Tags</span>
			<div class="tags">
				{#each realmDraft.tags as tag (tag)}
					<span class="tag">
						{tag}
						<button
							type="button"
							aria-label="Remove {tag}"
							onclick={() => (realmDraft.tags = realmDraft.tags.filter((t) => t !== tag))}
						>
							<X size={10} />
						</button>
					</span>
				{/each}
				<input
					class="tagInput plain"
					bind:value={tagDraft}
					placeholder="Add a tag…"
					aria-label="Add a tag"
					onkeydown={(e) => {
						if (e.key === 'Enter') {
							e.preventDefault();
							addTag();
						}
					}}
					onblur={addTag}
				/>
			</div>
		</div>

		<div class="field">
			<span>Modules</span>
			{#each realmDraft.modules as module, i (i)}
				<RadioField
					value={module}
					kinds={['module']}
					inputs={realmDraft.inputs}
					ariaLabel="Module {i + 1}"
					onchange={(next) => {
						if (next) realmDraft.modules[i] = next;
						else realmDraft.modules.splice(i, 1);
					}}
				/>
			{/each}
			<RadioField
				value={null}
				kinds={['module']}
				inputs={realmDraft.inputs}
				placeholder="Import a module…"
				ariaLabel="Import another module"
				onchange={(next) => next && realmDraft.modules.push(next)}
			/>
		</div>

		<div class="field">
			<span class="withIcon"><Shuffle size={13} /> Director</span>
			<select value={kind} onchange={(e) => setKind(e.currentTarget.value)}>
				<option value="round_robin">Round Robin</option>
				<option value="random">Random</option>
				<option value="script">Custom Script</option>
			</select>

			{#if kind === 'script'}
				<button
					type="button"
					class="scriptCard"
					class:blank={!scriptPreview}
					aria-label={scriptPreview ? 'Open the director script' : 'Write a director script'}
					onclick={() => panes.show({ kind: 'director' })}
				>
					{#if scriptPreview}
						<pre aria-hidden="true">{scriptPreview}</pre>
					{:else}
						<span class="blankText">Nothing written yet</span>
					{/if}
					<span class="openOverlay" aria-hidden="true">
						<Maximize2 size={12} />
						{scriptPreview ? 'Open' : 'Write one'}
					</span>
				</button>
				<p class="note">Should return the handle of whoever acts next.</p>
			{:else}
				<label class="sub">
					<span>Over</span>
					<select
						value={filterOf.match}
						onchange={(e) => {
							const match = e.currentTarget.value;
							setFilter(
								match === 'kind'
									? { match: 'kind', kind: 'llm' }
									: match === 'handles'
										? { match: 'handles', handles: [] }
										: { match: 'all' }
							);
						}}
					>
						<option value="all">Everyone</option>
						<option value="kind">A specific agent kind</option>
						<option value="handles">Only certain agents</option>
					</select>
				</label>

				{#if filterOf.match === 'kind'}
					<label class="sub">
						<span>Kind</span>
						<select
							value={filterOf.kind}
							onchange={(e) =>
								setFilter({ match: 'kind', kind: e.currentTarget.value as AgentKind })}
						>
							{#each AGENT_KINDS as k (k)}
								<option value={k}>{k}</option>
							{/each}
						</select>
					</label>
				{:else if filterOf.match === 'handles'}
					<label class="sub">
						<span>Handles</span>
						<input
							class="mono"
							value={filterOf.handles.join(', ')}
							placeholder="narrator, player"
							onchange={(e) =>
								setFilter({
									match: 'handles',
									handles: e.currentTarget.value
										.split(',')
										.map((h) => h.trim())
										.filter(Boolean)
								})}
						/>
					</label>
				{/if}
			{/if}
		</div>
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
		gap: var(--spacing-sm);
		height: var(--height-header);
		padding: 0 var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		align-items: center;
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
	}
	.body {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
		max-width: var(--width-content-max);
		overflow-y: auto;
	}
	label,
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	label > span,
	.field > span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.withIcon {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.sub {
		margin-left: var(--spacing-md);
	}
	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.tag {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 2px var(--spacing-xs) 2px var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.tag button {
		display: flex;
		padding: 0;
		background: transparent;
		border: none;
		color: var(--color-text-muted);
	}
	.tag button:hover {
		color: var(--color-error-text);
	}
	.tagInput {
		flex: 1 1 8em;
		min-width: 8em;
		background: transparent;
		border: none;
		font-size: var(--font-size-sm);
	}

	.scriptCard {
		position: relative;
		display: block;
		width: 100%;
		min-width: 0;
		padding: var(--spacing-sm);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		text-align: left;
		overflow: hidden;
	}
	.scriptCard:hover,
	.scriptCard:focus-visible {
		border-color: var(--color-border-active);
	}
	.scriptCard pre {
		margin: 0;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		color: var(--color-text-muted);
		white-space: pre;
		mask-image: linear-gradient(to right, #000 75%, transparent);
	}
	.scriptCard.blank {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 3.5em;
		border-style: dashed;
	}
	.blankText {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		font-style: italic;
	}
	.openOverlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		background-color: var(--color-bg-overlay);
		-webkit-backdrop-filter: blur(2px);
		backdrop-filter: blur(2px);
		color: var(--color-text-accent);
		font-size: var(--font-size-xs);
		opacity: 0;
		transition: opacity 120ms ease;
	}
	.scriptCard:hover .openOverlay,
	.scriptCard:focus-visible .openOverlay {
		opacity: 1;
	}
</style>
