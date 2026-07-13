<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { personaStore } from '$lib/stores/persona.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import type { AgentAccess, AgentKind, ToolBinding } from '$lib/types/realm.js';
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import { Plus, X } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let {
		oncreated,
		oncancel
	}: {
		oncreated?: () => void;
		oncancel?: () => void;
	} = $props();

	const api = useApi();

	let name = $state('');
	let kind = $state<AgentKind>('llm');
	let characterId = $state<string>('');
	let personaId = $state<string>('');
	// Open speech carries the `main` scope — without this grant the agent
	// never hears anyone. Prefilled; remove it to author a deaf agent.
	let grants = $state<Array<{ scope: string; access: AgentAccess }>>([
		{ scope: 'main', access: 'read' }
	]);
	/** tool_id -> event scope draft ('' = private) for each bound tool */
	let bindings = $state<Array<{ tool_id: string; event_scope: string }>>([]);
	let busy = $state(false);

	onMount(async () => {
		await Promise.all([
			characterStore.loadList(),
			personaStore.loadedPersonas ? Promise.resolve() : personaStore.loadPersonas(),
			toolStore.loadedTools ? Promise.resolve() : toolStore.loadTools()
		]);
	});

	function toggleTool(toolId: string) {
		const idx = bindings.findIndex((b) => b.tool_id === toolId);
		if (idx >= 0) bindings = bindings.filter((b) => b.tool_id !== toolId);
		else bindings = [...bindings, { tool_id: toolId, event_scope: '' }];
	}

	function boundIndex(toolId: string): number {
		return bindings.findIndex((b) => b.tool_id === toolId);
	}

	async function create() {
		const trimmed = name.trim();
		if (!trimmed || busy) return;
		busy = true;
		try {
			// create_agent wants character and version together; the version
			// is whatever the character currently points at.
			let character: { character_id: string; char_version_id: string } | null = null;
			if (kind === 'llm' && characterId) {
				const full = await api.characters.get(characterId);
				character = { character_id: full.id, char_version_id: full.current_version_id };
			}

			const tools: ToolBinding[] = bindings.map((b) => ({
				tool_id: b.tool_id,
				event_scope: b.event_scope.trim() === '' ? null : b.event_scope.trim()
			}));

			await realmStore.createAgent({
				name: trimmed,
				kind,
				...(character ?? {}),
				persona_id: personaId || null,
				grants: grants
					.filter((g) => g.scope.trim() !== '')
					.map((g) => ({ scope: g.scope.trim(), access: g.access })),
				tools
			});
			oncreated?.();
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), {
				title: 'Failed to create agent',
				durationMs: 5000
			});
		} finally {
			busy = false;
		}
	}
</script>

<form
	class="agentForm"
	onsubmit={(e) => {
		e.preventDefault();
		create();
	}}
>
	<label>
		<span>Name</span>
		<input type="text" bind:value={name} placeholder="Agent name" required />
	</label>

	<label>
		<span>Kind</span>
		<select bind:value={kind}>
			<option value="llm">LLM</option>
			<option value="player">Player (you)</option>
		</select>
	</label>

	{#if kind === 'llm'}
		<label>
			<span>Character</span>
			<select bind:value={characterId}>
				<option value="">None (template only)</option>
				{#each characterStore.metadataList as char (char.id)}
					<option value={char.id}>{char.display_name ?? char.name}</option>
				{/each}
			</select>
		</label>
	{/if}

	<label>
		<span>Persona</span>
		<select bind:value={personaId}>
			<option value="">None</option>
			{#each personaStore.loadedPersonas ?? [] as persona (persona.id)}
				<option value={persona.id}>{persona.display_name ?? persona.name}</option>
			{/each}
		</select>
	</label>

	<fieldset>
		<legend>
			State grants
			<button
				type="button"
				title="Grant a state scope"
				onclick={() => (grants = [...grants, { scope: '', access: 'read' }])}
			>
				<Plus size={12} />
			</button>
		</legend>
		{#each grants as grant, i (i)}
			<div class="row">
				<input type="text" bind:value={grant.scope} placeholder="scope, e.g. world.*" />
				<select bind:value={grant.access}>
					<option value="read">read</option>
					<option value="read_write">read/write</option>
				</select>
				<button
					type="button"
					title="Remove grant"
					onclick={() => (grants = grants.filter((_, j) => j !== i))}
				>
					<X size={12} />
				</button>
			</div>
		{:else}
			<div class="hint">
				No grants — the agent sees no world state and hears no open speech
				(speech carries the <code>main</code> scope).
			</div>
		{/each}
	</fieldset>

	<fieldset>
		<legend>Actions (bound tools)</legend>
		{#each toolStore.loadedTools ?? [] as tool (tool.id)}
			{@const idx = boundIndex(tool.id)}
			<div class="tool">
				<label class="toolToggle">
					<input type="checkbox" checked={idx >= 0} onchange={() => toggleTool(tool.id)} />
					<span>{tool.display_name ?? tool.name}</span>
				</label>
				{#if idx >= 0}
					<input
						type="text"
						class="scopeInput"
						bind:value={bindings[idx].event_scope}
						placeholder="event scope (empty = private)"
					/>
				{/if}
			</div>
		{:else}
			<div class="hint">No tools defined — the agent will be speak-only.</div>
		{/each}
	</fieldset>

	<div class="formActions">
		<button type="button" onclick={() => oncancel?.()}>Cancel</button>
		<button type="submit" class="primary" disabled={busy || !name.trim()}>
			{busy ? 'Creating…' : 'Create agent'}
		</button>
	</div>
</form>

<style>
	.agentForm {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-elevated);
		font-size: 0.9rem;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	label > span {
		font-size: 0.8rem;
		color: var(--color-text-secondary);
	}

	fieldset {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		padding: var(--spacing-sm);
		margin: 0;
	}

	legend {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: 0.8rem;
		color: var(--color-text-secondary);
		padding: 0 var(--spacing-xs);
	}

	legend button {
		display: flex;
		align-items: center;
		padding: 2px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.row input {
		flex: 1;
		min-width: 0;
	}

	.row select {
		width: auto;
		flex: 0 0 auto;
	}

	.row button {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
	}

	.tool {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.toolToggle {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.toolToggle input {
		width: auto;
	}

	.scopeInput {
		margin-left: var(--spacing-lg);
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.8rem;
	}

	.formActions {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-sm);
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
</style>
