<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { AgentKind, RealmAgent } from '$lib/types/realm.js';
	import { Bot, ChevronDown, ChevronUp, Terminal, Trash2, User, X } from '@lucide/svelte';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import WeaverField from '$lib/weaver/editor/WeaverField.svelte';
	import FieldShell from '$lib/components/FieldShell.svelte';
	import Badge from './Badge.svelte';
	import { builtinToolItems, catalogItem, realmEditor } from './realmEditor.svelte.js';

	/** Placeholder for property/override value inputs. */
	const VALUE_HINT = 'value — JSON, e.g. 3 or "shy"';

	let { agent, index, total }: { agent: RealmAgent; index: number; total: number } = $props();

	const KIND = {
		llm: { label: 'LLM', icon: Bot, tone: 'accent' as const },
		player: { label: 'Player', icon: User, tone: 'info' as const },
		script: { label: 'Script', icon: Terminal, tone: 'warning' as const }
	};

	const meta = $derived(KIND[agent.kind]);
	const expanded = $derived(realmEditor.expandedAgentId === agent.id);

	const imports = $derived(realmStore.realm?.definition.imports ?? []);
	const importedOf = (
		kind:
			| 'character'
			| 'template'
			| 'tool'
			| 'lorebook'
			| 'persona'
			| 'connection_profile'
			| 'widget'
	) =>
		imports
			.filter((r) => r.kind === kind)
			.map((r) => catalogItem(kind, r.resource_id))
			.filter((c) => c !== undefined);
	const characters = $derived(importedOf('character'));
	const templates = $derived(importedOf('template'));
	const tools = $derived(importedOf('tool'));
	const lorebooks = $derived(importedOf('lorebook'));
	const personas = $derived(importedOf('persona'));
	const profiles = $derived(importedOf('connection_profile'));
	const importedWidgets = $derived(importedOf('widget'));

	const builtins = $derived(builtinToolItems());
	const boundToolIds = $derived(new Set(agent.tools.map((t) => t.tool_id)));
	const availableTools = $derived(tools.filter((t) => !boundToolIds.has(t.id)));
	const availableBuiltins = $derived(builtins.filter((t) => !boundToolIds.has(t.id)));
	const isBuiltin = (id: Snowflake) => builtins.some((t) => t.id === id);
	const toolName = (id: Snowflake) =>
		catalogItem('tool', id)?.name ?? builtins.find((t) => t.id === id)?.name ?? id;
	let toolPick = $state('');

	const summary = $derived(
		agent.kind === 'script'
			? `${agent.grants.length} grants`
			: `${agent.speech_channel ? 'Private → ' + agent.speech_channel : 'Broadcast (main)'} · ${
					agent.tools.length === 0 ? 'speak-only' : agent.tools.length + ' tools'
				}`
	);

	const edit = (mutate: (a: RealmAgent) => void) => realmStore.editAgent(agent.id, mutate);

	/** Clear the fields a kind can't carry */
	function changeKind(kind: AgentKind) {
		edit((a) => {
			a.kind = kind;
			if (kind !== 'llm') {
				a.character_id = null;
				a.char_version_id = null;
				a.template_id = null;
			}
			if (kind === 'script') {
				a.profile_id = null;
				a.persona_id = null;
				a.tools = [];
				a.speech_channel = null;
				a.lorebooks = [];
				a.script = a.script ?? '';
			} else {
				a.script = null;
			}
		});
	}

	function bindTool() {
		if (!toolPick) return;
		const id = toolPick;
		toolPick = '';
		edit((a) => {
			if (!a.tools.some((t) => t.tool_id === id))
				a.tools = [...a.tools, { tool_id: id, event_channel: null }];
		});
	}

	function toggleLorebook(id: Snowflake) {
		edit((a) => {
			a.lorebooks = a.lorebooks.includes(id)
				? a.lorebooks.filter((l) => l !== id)
				: [...a.lorebooks, id];
		});
	}

	// Property values are JSON
	const displayValue = (v: unknown) => (typeof v === 'string' ? v : JSON.stringify(v));
	function parseValue(raw: string): unknown {
		if (raw === '') return '';
		try {
			return JSON.parse(raw);
		} catch {
			return raw;
		}
	}

	function addProperty() {
		edit((a) => {
			let i = 1;
			while (`key_${i}` in a.properties) i++;
			a.properties = { ...a.properties, [`key_${i}`]: '' };
		});
	}

	function renameProperty(from: string, to: string) {
		if (from === to) return;
		edit((a) => {
			const next: Record<string, unknown> = {};
			for (const [k, v] of Object.entries(a.properties)) next[k === from ? to : k] = v;
			a.properties = next as typeof a.properties;
		});
	}
</script>

<div class="card">
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="head" onclick={() => realmEditor.toggleAgent(agent.id)}>
		<div class="reorder">
			<button
				type="button"
				class="mini"
				title="Move earlier"
				disabled={index === 0}
				onclick={(e) => {
					e.stopPropagation();
					void realmStore.moveAgent(agent.id, -1);
				}}
			>
				<ChevronUp size={12} />
			</button>
			<button
				type="button"
				class="mini"
				title="Move later"
				disabled={index === total - 1}
				onclick={(e) => {
					e.stopPropagation();
					void realmStore.moveAgent(agent.id, 1);
				}}
			>
				<ChevronDown size={12} />
			</button>
		</div>
		<meta.icon size={18} color="var(--color-text-accent)" />
		<div class="name">{agent.name}</div>
		<Badge tone={meta.tone}>{meta.label}</Badge>
		<div class="spacer"></div>
		<div class="summary">{summary}</div>
		<button
			type="button"
			class="icon"
			title="Remove agent"
			onclick={(e) => {
				e.stopPropagation();
				void realmStore.deleteAgent(agent.id);
			}}
		>
			<Trash2 size={14} />
		</button>
		{#if expanded}<ChevronUp size={16} color="var(--color-text-secondary)" />{:else}<ChevronDown
				size={16}
				color="var(--color-text-secondary)"
			/>{/if}
	</div>

	{#if expanded}
		<div class="body">
			<div class="row2">
				<label>
					<span>Name</span>
					<input
						type="text"
						value={agent.name}
						oninput={(e) => edit((a) => (a.name = e.currentTarget.value))}
					/>
				</label>
				<label class="kind">
					<span>Handle</span>
					<input
						type="text"
						class="mono"
						value={agent.handle}
						placeholder="e.g. velen"
						oninput={(e) => edit((a) => (a.handle = e.currentTarget.value))}
					/>
				</label>
				<label class="kind">
					<span>Kind</span>
					<select
						value={agent.kind}
						onchange={(e) => changeKind(e.currentTarget.value as AgentKind)}
					>
						<option value="llm">LLM</option>
						<option value="player">Player</option>
						<option value="script">Script</option>
					</select>
				</label>
			</div>

			{#if agent.kind === 'llm'}
				<div class="row2">
					<label>
						<span>Character</span>
						<select
							value={agent.character_id ?? ''}
							onchange={(e) => void realmStore.setAgentCharacter(agent.id, e.currentTarget.value)}
						>
							<option value="">— none —</option>
							{#each characters as c (c.id)}
								<option value={c.id}>{c.name}</option>
							{/each}
						</select>
					</label>
					<label>
						<span>Context template</span>
						<select
							value={agent.template_id ?? ''}
							onchange={(e) => edit((a) => (a.template_id = e.currentTarget.value || null))}
						>
							<option value="">— none —</option>
							{#each templates as t (t.id)}
								<option value={t.id}>{t.name}</option>
							{/each}
						</select>
					</label>
				</div>
			{/if}

			{#if agent.kind !== 'script'}
				<div class="row2">
					<label>
						<span>Connection profile{agent.kind === 'player' ? ' (optional)' : ''}</span>
						<select
							value={agent.profile_id ?? ''}
							onchange={(e) => edit((a) => (a.profile_id = e.currentTarget.value || null))}
						>
							<option value="">— none —</option>
							{#each profiles as p (p.id)}
								<option value={p.id}>{p.name}</option>
							{/each}
						</select>
					</label>
					<label>
						<span>Persona (optional)</span>
						<select
							value={agent.persona_id ?? ''}
							onchange={(e) => edit((a) => (a.persona_id = e.currentTarget.value || null))}
						>
							<option value="">— none —</option>
							{#each personas as p (p.id)}
								<option value={p.id}>{p.name}</option>
							{/each}
						</select>
					</label>
				</div>
			{/if}

			{#if agent.kind !== 'script'}
				<label>
					<span>Speech scope</span>
					<input
						type="text"
						value={agent.speech_channel ?? ''}
						placeholder="Blank = broadcast on the main channel"
						oninput={(e) => edit((a) => (a.speech_channel = e.currentTarget.value || null))}
					/>
				</label>
			{/if}

			{#if agent.kind === 'script'}
				<FieldShell label="Weaver script">
					{#snippet editor(fill: boolean)}
						<div class="scriptBox" class:fill>
							<WeaverEditor
								doc={agent.script ?? ''}
								onChange={(v) => edit((a) => (a.script = v))}
							/>
						</div>
					{/snippet}
				</FieldShell>
			{/if}

			<div class="field">
				<span class="fieldLabel">Run condition <em>— optional, blank = always runs</em></span>
				<WeaverField
					doc={agent.run_condition ?? ''}
					onChange={(v) => edit((a) => (a.run_condition = v.trim() || null))}
				/>
			</div>

			<div class="section">
				<div class="sectionHead">
					<span>
						Properties <em>— seeded as agents:{agent.handle || '<handle>'}.key at play start</em>
					</span>
					<button type="button" class="ghost" onclick={addProperty}>+ Add property</button>
				</div>
				{#each Object.keys(agent.properties) as key (key)}
					<div class="grantRow">
						<input
							type="text"
							class="mono"
							value={key}
							placeholder="key (dotted path)"
							onchange={(e) => renameProperty(key, e.currentTarget.value)}
						/>
						<input
							type="text"
							class="mono"
							value={displayValue(agent.properties[key])}
							placeholder={VALUE_HINT}
							onchange={(e) =>
								edit(
									(a) =>
										(a.properties = {
											...a.properties,
											[key]: parseValue(e.currentTarget.value) as (typeof a.properties)[string]
										})
								)}
						/>
						<button
							type="button"
							class="icon"
							title="Remove property"
							onclick={() =>
								edit((a) => {
									const next = { ...a.properties };
									delete next[key];
									a.properties = next;
								})}
						>
							<X size={14} />
						</button>
					</div>
				{:else}
					<div class="hint">No properties — nothing seeds for this agent.</div>
				{/each}
			</div>

			<div class="section">
				<div class="sectionHead">
					<span>State grants</span>
					<button
						type="button"
						class="ghost"
						onclick={() => edit((a) => (a.grants = [...a.grants, { channel: '', access: 'read' }]))}
					>
						+ Add grant
					</button>
				</div>
				{#each agent.grants as grant, i (i)}
					<div class="grantRow">
						<input
							type="text"
							class="mono"
							value={grant.channel}
							placeholder="e.g. nation:velen.*"
							oninput={(e) => edit((a) => (a.grants[i].channel = e.currentTarget.value))}
						/>
						<select
							value={grant.access}
							onchange={(e) =>
								edit((a) => (a.grants[i].access = e.currentTarget.value as 'read' | 'read_write'))}
						>
							<option value="read">Read</option>
							<option value="read_write">Read/Write</option>
						</select>
						<button
							type="button"
							class="icon"
							title="Remove grant"
							onclick={() => edit((a) => (a.grants = a.grants.filter((_, j) => j !== i)))}
						>
							<X size={14} />
						</button>
					</div>
				{:else}
					<div class="hint">No grants — this agent can't see any state.</div>
				{/each}
			</div>

			{#if agent.kind !== 'script'}
				<div class="section">
					<div class="sectionLabel">Tools <em>— no tools bound means speak-only</em></div>
					<div class="hint">
						Binding a tool grants its full capability: tool bodies read and write the whole world
						state, and their output enters this agent's feed permanently.
					</div>
					{#each agent.tools as binding, i (binding.tool_id)}
						<div class="toolRow">
							<div class="toolName">
								{toolName(binding.tool_id)}
								{#if isBuiltin(binding.tool_id)}<Badge tone="info">builtin</Badge>{/if}
							</div>
							<input
								type="text"
								class="mono"
								value={binding.event_channel ?? ''}
								placeholder="Event scope — blank = private to the actor"
								oninput={(e) =>
									edit((a) => (a.tools[i].event_channel = e.currentTarget.value || null))}
							/>
							<button
								type="button"
								class="icon"
								title="Unbind tool"
								onclick={() =>
									edit((a) => (a.tools = a.tools.filter((t) => t.tool_id !== binding.tool_id)))}
							>
								<X size={14} />
							</button>
						</div>
					{/each}
					{#if availableTools.length > 0 || availableBuiltins.length > 0}
						<div class="bindRow">
							<select bind:value={toolPick}>
								<option value="">Add a tool…</option>
								{#if availableTools.length > 0}
									<optgroup label="Imported">
										{#each availableTools as t (t.id)}
											<option value={t.id}>{t.name}</option>
										{/each}
									</optgroup>
								{/if}
								{#if availableBuiltins.length > 0}
									<optgroup label="Builtin">
										{#each availableBuiltins as t (t.id)}
											<option value={t.id}>{t.name}</option>
										{/each}
									</optgroup>
								{/if}
							</select>
							<button type="button" class="secondary" onclick={bindTool}>Bind</button>
						</div>
					{/if}
				</div>
			{/if}

			{#if agent.kind === 'llm'}
				<div class="section">
					<div class="sectionLabel">Lorebooks</div>
					{#each lorebooks as lb (lb.id)}
						<label class="check">
							<input
								type="checkbox"
								checked={agent.lorebooks.includes(lb.id)}
								onchange={() => toggleLorebook(lb.id)}
							/>
							<span>{lb.name}</span>
						</label>
					{:else}
						<div class="hint">No lorebooks imported into this realm yet.</div>
					{/each}
				</div>
			{/if}

			{#if agent.kind !== 'script'}
				<div class="section">
					<div class="sectionLabel">
						Widgets <em>— shown when this agent is the viewer</em>
					</div>
					{#each importedWidgets as w (w.id)}
						<label class="check">
							<input
								type="checkbox"
								checked={agent.widgets.includes(w.id)}
								onchange={() =>
									edit((a) => {
										a.widgets = a.widgets.includes(w.id)
											? a.widgets.filter((id) => id !== w.id)
											: [...a.widgets, w.id];
									})}
							/>
							<span>{w.name}</span>
						</label>
					{:else}
						<div class="hint">No widgets imported into this realm yet.</div>
					{/each}
				</div>
			{/if}

			<div class="footer">
				<button type="button" class="danger" onclick={() => void realmStore.deleteAgent(agent.id)}>
					Delete agent
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		padding: var(--spacing-md);
	}

	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		cursor: pointer;
	}

	.reorder {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.mini {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 20px;
		height: 20px;
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: 0;
	}

	.mini:hover:not(:disabled) {
		color: var(--color-text-primary);
	}

	.name {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-base);
		flex-shrink: 0;
	}

	.spacer {
		flex: 1;
	}

	.summary {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		white-space: nowrap;
	}

	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: 4px;
	}

	.icon:hover {
		color: var(--color-text-primary);
	}

	.body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		margin-top: var(--spacing-lg);
		padding-top: var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
	}

	.row2 {
		display: flex;
		gap: var(--spacing-lg);
	}

	.row2 > label {
		flex: 1;
	}

	.row2 > label.kind {
		flex: 0 0 160px;
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	label > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.fieldLabel em {
		color: var(--color-text-muted);
		font-style: normal;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.fieldLabel {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.scriptBox {
		display: flex;
	}
	/* Fill the overlay; inline the editor keeps its own natural height. */
	.scriptBox.fill {
		flex: 1 1 auto;
		min-height: 0;
		height: 100%;
	}

	.mono {
		font-family: var(--font-family-mono);
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.sectionHead {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.sectionHead > span,
	.sectionLabel {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.sectionHead em,
	.sectionLabel em {
		color: var(--color-text-muted);
		font-style: normal;
	}

	.grantRow,
	.toolRow,
	.bindRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.grantRow > input,
	.toolRow > input,
	.bindRow > select {
		flex: 1;
		min-width: 0;
	}

	.grantRow > select {
		width: 130px;
		flex-shrink: 0;
	}

	.toolName {
		flex: 0 0 160px;
		font-size: var(--font-size-base);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.check {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.check input {
		width: auto;
	}

	.ghost {
		background: none;
		border: none;
		color: var(--color-text-accent);
		padding: 2px 6px;
	}

	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.footer {
		display: flex;
		justify-content: flex-end;
	}

	.danger {
		background: none;
		border: 1px solid var(--color-danger, #e5484d);
		color: var(--color-danger, #e5484d);
	}

	.danger:hover {
		background: var(--color-danger, #e5484d);
		color: var(--color-text-white);
	}
</style>
