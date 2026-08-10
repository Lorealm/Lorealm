<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { JsonValue, Snowflake } from '$lib/types/core.js';
	import type { Scenario } from '$lib/types/realm.js';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import ContextField from '$lib/components/ContextField.svelte';
	import FieldShell from '$lib/components/FieldShell.svelte';
	import { ChevronDown, ChevronUp, Trash2, X } from '@lucide/svelte';

	/** Placeholder for property/override value inputs. */
	const VALUE_HINT = 'value — JSON, e.g. 3 or "shy"';

	const scenarios = $derived(realmStore.realm?.definition.scenarios ?? []);
	const agents = $derived(realmStore.agents.filter((a) => a.handle));

	interface OverrideRow {
		handle: string;
		key: string;
		value: string;
	}

	/** Explicit-save draft of the expanded scenario. */
	interface Draft {
		id: Snowflake;
		name: string;
		description: string;
		seed_script: string;
		opening: string;
		overrides: OverrideRow[];
	}

	let expandedId = $state<Snowflake | null>(null);
	let draft = $state<Draft | null>(null);
	let saving = $state(false);
	let adding = $state(false);

	const displayValue = (v: JsonValue) => (typeof v === 'string' ? v : JSON.stringify(v));
	function parseValue(raw: string): JsonValue {
		if (raw === '') return '';
		try {
			return JSON.parse(raw) as JsonValue;
		} catch {
			return raw;
		}
	}

	function toDraft(s: Scenario): Draft {
		const overrides: OverrideRow[] = [];
		for (const [handle, props] of Object.entries(s.agent_overrides))
			for (const [key, value] of Object.entries(props))
				overrides.push({ handle, key, value: displayValue(value) });
		return {
			id: s.id,
			name: s.name,
			description: s.description,
			seed_script: s.seed_script,
			opening: s.opening ?? '',
			overrides
		};
	}

	function toggle(s: Scenario) {
		if (expandedId === s.id) {
			expandedId = null;
			draft = null;
			return;
		}
		expandedId = s.id;
		draft = toDraft(s);
	}

	async function save() {
		if (!draft || saving) return;
		saving = true;
		try {
			const agent_overrides: Record<string, Record<string, JsonValue>> = {};
			for (const row of draft.overrides) {
				if (!row.handle || !row.key) continue;
				(agent_overrides[row.handle] ??= {})[row.key] = parseValue(row.value);
			}
			await realmStore.updateScenario(draft.id, {
				name: draft.name,
				description: draft.description,
				seed_script: draft.seed_script,
				opening: draft.opening.trim() ? draft.opening : null,
				agent_overrides
			});
		} finally {
			saving = false;
		}
	}

	async function add() {
		if (adding) return;
		adding = true;
		try {
			const created = await realmStore.createScenario({
				name: `Scenario ${scenarios.length + 1}`
			});
			if (created) {
				expandedId = created.id;
				draft = toDraft(created);
			}
		} finally {
			adding = false;
		}
	}

	async function remove(id: Snowflake) {
		await realmStore.deleteScenario(id);
		if (expandedId === id) {
			expandedId = null;
			draft = null;
		}
	}
</script>

<div class="stage">
	<div class="toolbar">
		<button type="button" class="primary" onclick={add} disabled={adding}>+ Add scenario</button>
	</div>
	<div class="hint">
		A scenario is a named opening: its seed script writes the turn-zero world state (over the cast's
		seeded properties), its opening narration lands in every main-channel feed, and its overrides
		tweak agent properties for this opening only.
	</div>

	{#each scenarios as scenario (scenario.id)}
		<div class="card">
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div class="head" onclick={() => toggle(scenario)}>
				<div class="name">{scenario.name || 'Untitled scenario'}</div>
				<div class="spacer"></div>
				<div class="summary">{scenario.description}</div>
				<button
					type="button"
					class="icon"
					title="Delete scenario"
					onclick={(e) => {
						e.stopPropagation();
						void remove(scenario.id);
					}}
				>
					<Trash2 size={14} />
				</button>
				{#if expandedId === scenario.id}<ChevronUp
						size={16}
						color="var(--color-text-secondary)"
					/>{:else}<ChevronDown size={16} color="var(--color-text-secondary)" />{/if}
			</div>

			{#if expandedId === scenario.id && draft}
				<div class="body">
					<div class="row2">
						<label>
							<span>Name</span>
							<input type="text" bind:value={draft.name} />
						</label>
						<label>
							<span>Description</span>
							<input type="text" bind:value={draft.description} />
						</label>
					</div>

					<ContextField
						label="Opening narration"
						hint="The playthrough's prologue, heard on `main`."
						value={draft.opening}
						rows={4}
						onchange={(v) => (draft!.opening = v)}
					/>

					<FieldShell label="Seed script" hint="weaver, committed as the turn-zero world state.">
						{#snippet editor(fill: boolean)}
							<div class="scriptBox" class:fill>
								<WeaverEditor doc={draft!.seed_script} onChange={(v) => (draft!.seed_script = v)} />
							</div>
						{/snippet}
					</FieldShell>

					<div class="section">
						<div class="sectionHead">
							<span>
								Agent overrides <em>— replace authored properties for this opening</em>
							</span>
							<button
								type="button"
								class="ghost"
								disabled={agents.length === 0}
								onclick={() =>
									(draft!.overrides = [
										...draft!.overrides,
										{ handle: agents[0]?.handle ?? '', key: '', value: '' }
									])}
							>
								+ Add override
							</button>
						</div>
						{#each draft.overrides as row, i (i)}
							<div class="overrideRow">
								<select bind:value={row.handle}>
									{#each agents as a (a.id)}
										<option value={a.handle}>{a.name} ({a.handle})</option>
									{/each}
								</select>
								<input type="text" class="mono" placeholder="key" bind:value={row.key} />
								<input type="text" class="mono" placeholder={VALUE_HINT} bind:value={row.value} />
								<button
									type="button"
									class="icon"
									title="Remove override"
									onclick={() => (draft!.overrides = draft!.overrides.filter((_, j) => j !== i))}
								>
									<X size={14} />
								</button>
							</div>
						{:else}
							<div class="hint">No overrides — agents keep their authored properties.</div>
						{/each}
						{#if agents.length === 0}
							<div class="hint">Overrides need cast agents with handles.</div>
						{/if}
					</div>

					<div class="footer">
						<button type="button" class="primary" onclick={save} disabled={saving}>
							{saving ? 'Saving…' : 'Save scenario'}
						</button>
					</div>
				</div>
			{/if}
		</div>
	{:else}
		<div class="empty">
			No scenarios — playthroughs start empty and the world emerges from lorebooks and tools.
		</div>
	{/each}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	.toolbar {
		display: flex;
		justify-content: flex-end;
	}

	.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}

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

	.name {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-base);
	}

	.spacer {
		flex: 1;
	}

	.summary {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 50%;
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

	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	label > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.sectionHead em {
		color: var(--color-text-muted);
		font-style: normal;
	}

	.scriptBox {
		display: flex;
	}
	
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

	.sectionHead > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.overrideRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.overrideRow > select {
		width: 200px;
		flex-shrink: 0;
	}

	.overrideRow > input {
		flex: 1;
		min-width: 0;
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

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}

	.footer {
		display: flex;
		justify-content: flex-end;
	}
</style>
