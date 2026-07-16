<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import { personaStore } from '$lib/stores/persona.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { studioStore, type StudioResourceKind } from '$lib/stores/studio.svelte.js';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import type { AgentAccess, AgentKind, RealmAgent, ToolBinding } from '$lib/types/realm.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import { Plus, SquarePen, X } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let {
		agent = null,
		ondone,
		oncancel
	}: {
		/** Edit this preset agent in place; omit to create a new one. */
		agent?: RealmAgent | null;
		/** Called with the saved agent. */
		ondone?: (saved: RealmAgent) => void;
		oncancel?: () => void;
	} = $props();

	const api = useApi();

	// svelte-ignore state_referenced_locally
	const initial = agent;

	let name = $state(initial?.name ?? '');
	let kind = $state<AgentKind>(initial?.kind ?? 'llm');
	let characterId = $state<string>(initial?.character_id ?? '');
	let templateId = $state<string>(initial?.template_id ?? '');
	let personaId = $state<string>(initial?.persona_id ?? '');
	let profileId = $state<string>(initial?.profile_id ?? '');
	// Speech on `main` is only heard by agents granted read on `main`, so new
	// agents are prefilled with that grant.
	let grants = $state<Array<{ scope: string; access: AgentAccess }>>(
		initial ? initial.grants.map((g) => ({ ...g })) : [{ scope: 'main', access: 'read' }]
	);
	let bindings = $state<Array<{ tool_id: string; event_scope: string }>>(
		initial
			? initial.tools.map((b) => ({ tool_id: b.tool_id, event_scope: b.event_scope ?? '' }))
			: []
	);
	let lorebooks = $state<string[]>(initial ? [...(initial.lorebooks ?? [])] : []);
	let speechScope = $state<string>(initial?.speech_scope ?? '');
	let script = $state<string>(initial?.script ?? '');
	// Optional weaver expression gating whether this agent takes its turn
	// ('' = always runs). Any kind may carry one.
	let runCondition = $state<string>(initial?.run_condition ?? '');
	let busy = $state(false);

	// Strict membrane: only imported resources are offered.
	const imports = $derived(realmStore.realm?.definition.imports ?? []);
	const importedCharacters = $derived(
		(characterStore.metadataList ?? []).filter((c) =>
			imports.some((r) => r.kind === 'character' && r.resource_id === c.id)
		)
	);
	const importedTemplates = $derived(
		(templateStore.metadataList ?? []).filter((t) =>
			imports.some((r) => r.kind === 'template' && r.resource_id === t.id)
		)
	);
	const importedTools = $derived(
		(toolStore.loadedTools ?? []).filter((t) =>
			imports.some((r) => r.kind === 'tool' && r.resource_id === t.id)
		)
	);
	const importedLorebooks = $derived(
		(lorebookStore.metadataList ?? []).filter((l) =>
			imports.some((r) => r.kind === 'lorebook' && r.resource_id === l.id)
		)
	);

	onMount(async () => {
		await Promise.all([
			characterStore.metadataList ? Promise.resolve() : characterStore.loadList(),
			lorebookStore.metadataList ? Promise.resolve() : lorebookStore.loadList(),
			templateStore.metadataList ? Promise.resolve() : templateStore.loadList(),
			personaStore.loadedPersonas ? Promise.resolve() : personaStore.loadPersonas(),
			toolStore.loadedTools ? Promise.resolve() : toolStore.loadTools(),
			connectionStore.profiles ? Promise.resolve() : connectionStore.loadProfiles()
		]);
		// New LLM agents bind every imported lorebook by default.
		if (!agent && lorebooks.length === 0) {
			lorebooks = importedLorebooks.map((l) => l.id);
		}
	});

	const canSubmit = $derived(
		name.trim() !== '' &&
			!busy &&
			(kind === 'player'
				? true
				: kind === 'script'
					? script.trim() !== ''
					: templateId !== '')
	);

	function toggleTool(toolId: string) {
		const idx = bindings.findIndex((b) => b.tool_id === toolId);
		if (idx >= 0) bindings = bindings.filter((b) => b.tool_id !== toolId);
		else bindings = [...bindings, { tool_id: toolId, event_scope: '' }];
	}

	function boundIndex(toolId: string): number {
		return bindings.findIndex((b) => b.tool_id === toolId);
	}

	function toggleLorebook(id: string) {
		if (lorebooks.includes(id)) lorebooks = lorebooks.filter((l) => l !== id);
		else lorebooks = [...lorebooks, id];
	}

	function jump(kind: StudioResourceKind, id: string) {
		if (id) studioStore.editResource(kind, id);
	}

	async function submit() {
		const trimmed = name.trim();
		if (!canSubmit || !trimmed) return;
		busy = true;
		try {
			// create/update want character and version together. Keep the
			// pinned version when the character didn't change; a new pick
			// pins whatever the character currently points at.
			let character: { character_id: string; char_version_id: string } | null = null;
			if (kind === 'llm' && characterId) {
				if (agent && agent.character_id === characterId && agent.char_version_id) {
					character = { character_id: characterId, char_version_id: agent.char_version_id };
				} else {
					const full = await api.characters.get(characterId);
					character = { character_id: full.id, char_version_id: full.current_version_id };
				}
			}

			const tools: ToolBinding[] = bindings.map((b) => ({
				tool_id: b.tool_id,
				event_scope: b.event_scope.trim() === '' ? null : b.event_scope.trim()
			}));

			const request = {
				name: trimmed,
				kind,
				...(character ?? {}),
				template_id: kind === 'llm' && templateId ? templateId : null,
				profile_id: kind === 'llm' && profileId ? profileId : null,
				persona_id: kind === 'script' ? null : personaId || null,
				grants: grants
					.filter((g) => g.scope.trim() !== '')
					.map((g) => ({ scope: g.scope.trim(), access: g.access })),
				// Script agents can't act or speak; only llm/player carry a
				// speech scope, and only non-script agents bind tools.
				tools: kind === 'script' ? [] : tools,
				speech_scope: kind === 'script' ? null : speechScope.trim() || null,
				script: kind === 'script' ? script : null,
				run_condition: runCondition.trim() || null,
				lorebooks: kind === 'llm' ? (lorebooks as Snowflake[]) : []
			};

			const saved = agent
				? await realmStore.updateAgent(agent.id, request)
				: await realmStore.createAgent(request);
			if (saved) ondone?.(saved);
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), {
				title: agent ? 'Failed to update agent' : 'Failed to create agent',
				durationMs: 5000
			});
		} finally {
			busy = false;
		}
	}
</script>

<form
	class="editor"
	onsubmit={(e) => {
		e.preventDefault();
		submit();
	}}
>
	<section>
		<h4>Identity</h4>
		<div class="grid">
			<label>
				<span>Name</span>
				<input type="text" bind:value={name} placeholder="Agent name" required />
			</label>

			<label>
				<span>Kind</span>
				<select bind:value={kind} disabled={agent !== null}>
					<option value="llm">LLM</option>
					<option value="player">Player (you)</option>
					<option value="script">Script (weaver)</option>
				</select>
			</label>

			{#if kind === 'llm'}
				<label>
					<span>Character</span>
					<div class="pickerRow">
						<select bind:value={characterId}>
							<option value="">None (template only)</option>
							{#each importedCharacters as char (char.id)}
								<option value={char.id}>{char.display_name ?? char.name}</option>
							{/each}
						</select>
						{#if characterId}
							<button
								type="button"
								class="jump"
								title="Edit this character in the Studio"
								onclick={() => jump('character', characterId)}
							>
								<SquarePen size={14} />
							</button>
						{/if}
					</div>
					{#if importedCharacters.length === 0}
						<span class="hint">No characters imported into this realm.</span>
					{/if}
				</label>

				<label>
					<span>Context template</span>
					<div class="pickerRow">
						<select bind:value={templateId} required>
							<option value="" disabled>Pick an imported template</option>
							{#each importedTemplates as template (template.id)}
								<option value={template.id}>{template.name}</option>
							{/each}
						</select>
						{#if templateId}
							<button
								type="button"
								class="jump"
								title="Edit this template in the Studio"
								onclick={() => jump('template', templateId)}
							>
								<SquarePen size={14} />
							</button>
						{/if}
					</div>
					{#if importedTemplates.length === 0}
						<span class="hint">Import a context template first — LLM agents need one.</span>
					{/if}
				</label>

				<label>
					<span>Connection profile</span>
					<select bind:value={profileId}>
						<option value="">Your active profile (default)</option>
						{#each connectionStore.profiles ?? [] as profile (profile.id)}
							<option value={profile.id}>{profile.name}</option>
						{/each}
					</select>
				</label>
			{/if}

			{#if kind !== 'script'}
				<label>
					<span>Persona</span>
					<div class="pickerRow">
						<select bind:value={personaId}>
							<option value="">None</option>
							{#each personaStore.loadedPersonas ?? [] as persona (persona.id)}
								<option value={persona.id}>{persona.display_name ?? persona.name}</option>
							{/each}
						</select>
						{#if personaId}
							<button
								type="button"
								class="jump"
								title="Edit this persona in the Studio"
								onclick={() => jump('persona', personaId)}
							>
								<SquarePen size={14} />
							</button>
						{/if}
					</div>
				</label>

				<label>
					<span>Speech scope</span>
					<input type="text" bind:value={speechScope} placeholder="main (broadcast)" />
					<span class="hint">Who hears this agent. Blank = the broadcast <code>main</code> channel.</span>
				</label>
			{/if}
		</div>
	</section>

	{#if kind === 'script'}
		<section>
			<h4>Script (weaver)</h4>
			<textarea
				class="script"
				bind:value={script}
				rows="8"
				placeholder="Weaver source run each turn against this agent's granted state, e.g.&#10;state.set('log.events', 'entries', [])"
			></textarea>
			<span class="hint">
				Runs deterministically on the agent's turn — no model, no speech. Use it to
				maintain state (trim an event log, advance a calendar). Its writes are gated by
				the read/write grants below.
			</span>
		</section>
	{/if}

	<section>
		<h4>
			State grants
			<button
				type="button"
				class="add"
				title="Grant a state scope"
				onclick={() => (grants = [...grants, { scope: '', access: 'read' }])}
			>
				<Plus size={12} />
			</button>
		</h4>
		{#each grants as grant, i (i)}
			<div class="row">
				<input type="text" bind:value={grant.scope} placeholder="scope, e.g. world.*" />
				<select bind:value={grant.access}>
					<option value="read">read</option>
					<option value="read_write">read/write</option>
				</select>
				<button
					type="button"
					class="remove"
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
	</section>

	<section>
		<h4>Run condition</h4>
		<input
			type="text"
			class="condition"
			bind:value={runCondition}
			placeholder={'{{state:turn}} >= 7'}
		/>
		<span class="hint">
			An optional <code>weaver</code> expression gating whether this agent takes its turn,
			evaluated against its granted read slice above. Blank = always runs. When
			false, the turn skips forward to the next agent whose condition passes. A
			broken or erroring expression never runs.
		</span>
	</section>

	{#if kind !== 'script'}
	<section>
		<h4>Actions (bound tools)</h4>
		{#each importedTools as tool (tool.id)}
			{@const idx = boundIndex(tool.id)}
			<div class="tool">
				<div class="pickerRow">
					<label class="toggle">
						<input type="checkbox" checked={idx >= 0} onchange={() => toggleTool(tool.id)} />
						<span>{tool.display_name ?? tool.name}</span>
					</label>
					<button
						type="button"
						class="jump"
						title="Edit this tool in the Studio"
						onclick={() => jump('tool', tool.id)}
					>
						<SquarePen size={14} />
					</button>
				</div>
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
			<div class="hint">No tools imported — the agent will be speak-only.</div>
		{/each}
	</section>
	{/if}

	{#if kind === 'llm'}
		<section>
			<h4>Lorebooks</h4>
			{#each importedLorebooks as lorebook (lorebook.id)}
				<div class="pickerRow">
					<label class="toggle">
						<input
							type="checkbox"
							checked={lorebooks.includes(lorebook.id)}
							onchange={() => toggleLorebook(lorebook.id)}
						/>
						<span>{lorebook.name}</span>
					</label>
					<button
						type="button"
						class="jump"
						title="Edit this lorebook in the Studio"
						onclick={() => jump('lorebook', lorebook.id)}
					>
						<SquarePen size={14} />
					</button>
				</div>
			{:else}
				<div class="hint">No lorebooks imported — the agent gets no lorebook context.</div>
			{/each}
		</section>
	{/if}

	<div class="formActions">
		{#if oncancel}
			<button type="button" onclick={() => oncancel?.()}>Cancel</button>
		{/if}
		<button type="submit" class="primary" disabled={!canSubmit}>
			{busy ? 'Saving…' : agent ? 'Save agent' : 'Create agent'}
		</button>
	</div>
</form>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		padding: var(--spacing-md);
	}

	h4 {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin: 0;
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-secondary);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: var(--spacing-sm) var(--spacing-md);
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 0.9rem;
	}

	label > span {
		font-size: 0.8rem;
		color: var(--color-text-secondary);
	}

	.pickerRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.pickerRow select {
		flex: 1;
		min-width: 0;
	}

	.jump {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.jump:hover {
		color: var(--color-text-primary, inherit);
	}

	.add {
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

	.remove {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
	}

	.tool {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.toggle {
		flex: 1;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.toggle input {
		width: auto;
	}

	.scopeInput {
		margin-left: var(--spacing-lg);
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.8rem;
	}

	.script {
		width: 100%;
		resize: vertical;
		font-family: var(--font-mono, monospace);
		font-size: 0.85rem;
	}

	.condition {
		width: 100%;
		font-family: var(--font-mono, monospace);
		font-size: 0.85rem;
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
