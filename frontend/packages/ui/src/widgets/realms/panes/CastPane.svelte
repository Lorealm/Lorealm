<script lang="ts">
	import {
		Bot,
		FlagOff,
		GripVertical,
		Plus,
		Repeat,
		ScrollText,
		Trash2,
		User
	} from '@lucide/svelte';
	import type { AgentKind } from '$lib/types/realm.js';
	import { activeDrag, carriesAsset, clearDrag, readDrag } from '../drag.svelte.js';
	import { realmDraft } from '../draft.svelte.js';
	import { elementKind, type AgentBlock, type CastEntry } from '../model.js';

	/** Reordering stays inside the pane, so it gets its own mime. */
	const ENTRY_MIME = 'application/x-lorealm-cast-entry';

	const KIND_ICON = { llm: Bot, player: User, script: ScrollText } as const;

	let dragKey = $state<string | null>(null);
	/** Where the dragged entry would land */
	let dropIndex = $state<number | null>(null);
	let overEntry = $state<string | null>(null);

	const walkable = $derived(realmDraft.inputs.filter((i) => i.ty.type === 'list'));

	const incoming = $derived(activeDrag());
	const takesIncoming = $derived(
		incoming?.drag === 'input' && walkable.some((i) => i.name === incoming.name)
	);

	function startDrag(event: DragEvent, key: string) {
		dragKey = key;
		event.dataTransfer?.setData(ENTRY_MIME, key);
		if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
	}

	function overRow(event: DragEvent, index: number) {
		if (!dragKey) return;
		event.preventDefault();
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		dropIndex = event.clientY < box.top + box.height / 2 ? index : index + 1;
	}

	function finishDrag() {
		if (dragKey !== null && dropIndex !== null) realmDraft.moveEntry(dragKey, dropIndex);
		dragKey = null;
		dropIndex = null;
	}

	/** Alt+Arrow moves an entry without a mouse. */
	function onEntryKeydown(event: KeyboardEvent, key: string) {
		if (!event.altKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) return;
		event.preventDefault();
		const at = realmDraft.cast.findIndex((e) => e.key === key);
		realmDraft.moveEntry(key, event.key === 'ArrowUp' ? at - 1 : at + 2);
	}

	function bindOver(entry: CastEntry & { kind: 'foreach' }, name: string | null) {
		entry.over = name;
		realmDraft.clearStaleBindings(entry.key);
	}

	function dropInput(event: DragEvent, entry: CastEntry & { kind: 'foreach' }) {
		overEntry = null;
		clearDrag();
		const payload = readDrag(event);
		if (payload?.drag !== 'input') return;
		event.preventDefault();
		const decl = realmDraft.inputs.find((i) => i.name === payload.name);
		if (decl?.ty.type !== 'list') return;
		bindOver(entry, decl.name);
	}
</script>

{#snippet agentRow(agent: AgentBlock, nested: boolean)}
	{@const Icon = KIND_ICON[agent.kind as AgentKind]}
	<div class="row" class:nested>
		<button
			type="button"
			class="agent"
			class:selected={realmDraft.isSelected({ kind: 'agent', key: agent.key })}
			onclick={() => realmDraft.select({ kind: 'agent', key: agent.key })}
		>
			<Icon size={13} />
			<span class="handle">{agent.handle || 'unnamed'}</span>
			{#if agent.name}<span class="display">{agent.name}</span>{/if}
			<span class="kind">{agent.kind}</span>
		</button>
		<button
			type="button"
			class="remove"
			title="Remove agent"
			aria-label="Remove {agent.handle}"
			onclick={() => realmDraft.removeAgent(agent.key)}
		>
			<Trash2 size={12} />
		</button>
	</div>
{/snippet}

<div class="pane">
	<header>
		<h2>Cast</h2>
		<span class="order">turns run top-to-bottom</span>
		<button type="button" class="chip" onclick={() => realmDraft.addAgent()}>
			<Plus size={12} /> Agent
		</button>
		<button type="button" class="chip" onclick={() => realmDraft.addForeach()}>
			<Repeat size={12} /> For each
		</button>
	</header>

	<div class="stack" role="list" ondragend={finishDrag} ondragover={(e) => e.preventDefault()}>
		{#each realmDraft.cast as entry, index (entry.key)}
			{#if dropIndex === index}<div class="insertion"></div>{/if}
			<div
				class="entry"
				class:dragging={dragKey === entry.key}
				role="listitem"
				draggable="true"
				ondragstart={(e) => startDrag(e, entry.key)}
				ondragover={(e) => overRow(e, index)}
				ondrop={finishDrag}
			>
				<button
					type="button"
					class="grip"
					title="Drag to reorder, or Alt+Arrow"
					aria-label="Reorder entry {index + 1} of {realmDraft.cast.length}"
					onkeydown={(e) => onEntryKeydown(e, entry.key)}
				>
					<GripVertical size={12} />
				</button>

				<div class="entryBody">
					{#if entry.kind === 'agent'}
						{@render agentRow(entry.agent, false)}
					{:else}
						{@const decl = realmDraft.inputs.find((i) => i.name === entry.over)}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="loopHead"
							class:selected={realmDraft.isSelected({ kind: 'foreach', key: entry.key })}
							class:dropping={overEntry === entry.key}
							class:candidate={takesIncoming}
							class:incompatible={incoming !== null && !takesIncoming}
							ondragover={(e) => {
								if (!carriesAsset(e) || !takesIncoming) return;
								e.preventDefault();
								overEntry = entry.key;
							}}
							ondragleave={() => (overEntry = null)}
							ondrop={(e) => dropInput(e, entry)}
						>
							<button
								type="button"
								class="loopSelect"
								onclick={() => realmDraft.select({ kind: 'foreach', key: entry.key })}
							>
								<Repeat size={13} />
								<span class="kw">for each</span>
							</button>
							<input
								class="var"
								value={entry.varName}
								aria-label="Loop variable"
								spellcheck="false"
								oninput={(e) => (entry.varName = e.currentTarget.value)}
							/>
							<span class="kw">in</span>
							<select
								class="over"
								aria-label="Input to walk"
								value={entry.over ?? ''}
								onchange={(e) => bindOver(entry, e.currentTarget.value || null)}
							>
								<option value="">— pick an input —</option>
								{#each walkable as input (input.name)}
									<option value={input.name}>{input.name}</option>
								{/each}
							</select>
							{#if decl && !elementKind(decl.ty)}
								<span
									class="warn"
									title="Its elements aren't references. Only &lbrace;&lbrace;{entry.varName}&rbrace;&rbrace; can be reached."
								>
									<FlagOff size={12} />
								</span>
							{/if}
							<button
								type="button"
								class="remove"
								title="Remove loop"
								aria-label="Remove loop"
								onclick={() => realmDraft.removeEntry(entry.key)}
							>
								<Trash2 size={12} />
							</button>
						</div>

						<div class="body">
							{#each entry.body as agent (agent.key)}
								{@render agentRow(agent, true)}
							{/each}
							<button
								type="button"
								class="addMember"
								onclick={() => realmDraft.addToBody(entry.key)}
							>
								<Plus size={12} /> Agent per {entry.varName}
							</button>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="empty">
				<p>The cast is empty. Add one:</p>
				<div class="emptyActions">
					<button type="button" class="chip" onclick={() => realmDraft.addAgent()}>
						<Plus size={12} /> Agent
					</button>
					<button type="button" class="chip" onclick={() => realmDraft.addForeach()}>
						<Repeat size={12} /> For each
					</button>
				</div>
			</div>
		{/each}
		{#if dropIndex === realmDraft.cast.length}<div class="insertion"></div>{/if}
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
		align-items: center;
		gap: var(--spacing-sm);
		height: var(--height-header);
		padding: 0 var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
	}
	.order {
		flex: 1 1 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.chip {
		display: inline-flex;
		flex: 0 0 auto;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 3px var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.chip:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}

	.stack {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		gap: 2px;
		padding: var(--spacing-sm);
		overflow-y: auto;
	}

	.entry {
		display: flex;
		align-items: stretch;
		gap: var(--spacing-xs);
		border-radius: var(--radius-md);
	}
	.entry.dragging {
		opacity: 0.4;
	}
	.entry:has(.grip:focus-visible) {
		outline: 1px solid var(--color-border-active);
	}
	.grip {
		display: flex;
		align-items: center;
		flex: 0 0 auto;
		padding: 0;
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
		cursor: grab;
	}
	.entry:hover .grip {
		color: var(--color-text-muted);
	}
	.entryBody {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		gap: 2px;
	}

	.insertion {
		height: 2px;
		margin: 0 var(--spacing-sm);
		background-color: var(--color-primary-light);
		border-radius: var(--radius-full);
	}

	.row {
		display: flex;
		align-items: center;
		min-width: 0;
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.row.nested {
		margin-left: var(--spacing-lg);
	}
	.row:hover {
		border-color: var(--color-border-active);
	}
	.agent {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		text-align: left;
	}
	.agent.selected {
		background-color: var(--color-selected);
		color: var(--color-text-accent);
	}
	.handle {
		flex: 0 0 auto;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.display {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.kind {
		flex: 0 0 auto;
		margin-left: auto;
		padding: 1px 6px;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.remove {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		padding: 0 var(--spacing-sm);
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.remove:hover {
		color: var(--color-error-text);
	}

	.loopHead {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm) var(--spacing-xs) var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-sm);
	}
	.loopHead.selected {
		border-color: var(--color-border-active);
		background-color: var(--color-selected);
	}
	.loopHead.candidate {
		border-color: var(--color-border-active);
		border-style: dashed;
	}
	.loopHead.incompatible {
		opacity: 0.35;
		filter: grayscale(1);
	}
	.loopHead.dropping {
		border-color: var(--color-primary-light);
		border-style: solid;
		box-shadow: var(--shadow-glow);
	}
	.loopSelect {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 2px var(--spacing-xs);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
	}
	.kw {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.var {
		width: 6em;
		padding: 1px var(--spacing-xs);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
	}
	.over {
		max-width: 14em;
		padding: 1px var(--spacing-xs);
		font-size: var(--font-size-sm);
	}
	.warn {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		color: var(--color-warning-text);
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding-left: var(--spacing-sm);
		border-left: 1px dashed var(--color-border-medium);
		margin-left: var(--spacing-sm);
	}
	.addMember {
		align-self: flex-start;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin-left: var(--spacing-lg);
		padding: 2px var(--spacing-sm);
		background: transparent;
		border: 1px dashed var(--color-border-medium);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.addMember:hover {
		border-color: var(--color-border-active);
		color: var(--color-text-accent);
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-md);
		margin: var(--spacing-lg) auto;
		max-width: 28em;
		text-align: center;
	}
	.empty p {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.emptyActions {
		display: flex;
		gap: var(--spacing-sm);
	}
</style>
