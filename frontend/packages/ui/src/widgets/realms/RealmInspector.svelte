<script lang="ts">
	import { ExternalLink, Trash2 } from '@lucide/svelte';
	import ContextField from '$lib/components/ContextField.svelte';
	import { navigator } from '$lib/navigation/navigator.js';
	import type { StudioResource } from '$lib/navigation/navigator.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { AgentKind, RealmResourceKind } from '$lib/types/realm.js';
	import RadioField from './RadioField.svelte';
	import { realmDraft } from './draft.svelte.js';
	import { activeDrag, clearDrag, readDrag } from './drag.svelte.js';
	import {
		baseOf,
		declarationLine,
		declaredText,
		elementKind,
		holdsToBase,
		holdsValue,
		isList,
		referencedKind,
		isRequired,
		withList,
		type AgentBlock,
		type Binding,
		type InputDecl,
		type InputType
	} from './model.js';
	import { RESOURCE_KINDS } from './resources.svelte.js';

	const STUDIO_KINDS: Partial<Record<RealmResourceKind, StudioResource>> = {
		character: 'character',
		lorebook: 'lorebook',
		template: 'template',
		tool: 'tool',
		persona: 'persona',
		widget: 'widget'
	};

	const AGENT_KINDS: AgentKind[] = ['llm', 'player', 'script'];

	const selection = $derived(realmDraft.selection);
	const agent = $derived(selection?.kind === 'agent' ? realmDraft.agent(selection.key) : null);
	const loop = $derived(
		selection?.kind === 'foreach'
			? realmDraft.cast.find((e) => e.key === selection.key && e.kind === 'foreach')
			: undefined
	);
	const input = $derived(
		selection?.kind === 'input'
			? realmDraft.inputs.find((i) => i.name === selection.name)
			: undefined
	);
	const resource = $derived(selection?.kind === 'resource' ? selection : undefined);

	const scope = $derived(agent ? realmDraft.scopeOf(agent.key) : []);
	const inputs = $derived(realmDraft.inputs);
	const loopVar = $derived(scope[0]?.varName ?? null);
	const incoming = $derived(activeDrag());

	/**
	 * How an input reads inside a string field, or null where it cannot be read
	 * at all.
	 */
	function interpolation(name: string, field: 'slug' | 'name'): string | null {
		const looped = scope.find((v) => v.input?.name === name);
		if (looped) return `{{${looped.varName}.${field}}}`;
		const decl = inputs.find((i) => i.name === name);
		if (!decl || isList(decl.ty)) return null;
		return referencedKind(decl.ty) ? `{{input:${name}.${field}}}` : `{{input:${name}}}`;
	}

	function textTakes(field: 'slug' | 'name'): boolean {
		return incoming?.drag === 'input' && interpolation(incoming.name, field) !== null;
	}

	/** Splice the interpolation in at the caret rather than replacing the field. */
	function dropIntoText(event: DragEvent, field: 'slug' | 'name', apply: (next: string) => void) {
		const payload = readDrag(event);
		clearDrag();
		if (payload?.drag !== 'input') return;
		const snippet = interpolation(payload.name, field);
		if (!snippet) return;
		event.preventDefault();
		const el = event.currentTarget as HTMLInputElement;
		const from = el.selectionStart ?? el.value.length;
		const to = el.selectionEnd ?? from;
		apply(el.value.slice(0, from) + snippet + el.value.slice(to));
	}

	/** Leave the editor for wherever a resource is really edited. */
	function openResource(kind: RealmResourceKind, id: Snowflake) {
		if (kind === 'connection_profile') {
			navigator.start().connections(id);
			return;
		}
		const studio = STUDIO_KINDS[kind];
		if (studio) navigator.start().studio().resource(studio, id);
	}

	/**
	 * The Unity-like ping. Handles navigating to the source of an object when clicking it
	 * in a Radio field.
	 */
	function locate(binding: Binding, mode: 'select' | 'open') {
		if (binding.source === 'input') {
			realmDraft.select({ kind: 'input', name: binding.name });
			return;
		}
		if (binding.source === 'loop') {
			const found = scope.find((v) => v.varName === binding.varName);
			if (found?.input) realmDraft.select({ kind: 'input', name: found.input.name });
			return;
		}
		if (mode === 'open') {
			openResource(binding.kind, binding.id);
			return;
		}
		realmDraft.select({
			kind: 'resource',
			resourceKind: binding.kind,
			id: binding.id,
			name: binding.name
		});
	}

	/** An agent's shape depends on its kind; these say which fields mean anything. */
	function shows(a: AgentBlock, field: 'character' | 'template' | 'persona' | 'script') {
		if (field === 'script') return a.kind === 'script';
		if (field === 'character') return a.kind === 'llm';
		if (field === 'persona') return a.kind === 'player';
		return a.kind !== 'script';
	}

	/**
	 * Retype a declaration, dropping a default that the new type can't hold
	 */
	function retype(decl: InputDecl, base: InputType, many: boolean) {
		const previous = baseOf(decl.ty);
		realmDraft.retypeInput(decl.name, withList(base, many));
		if (many || base.type === 'reference' || base.type !== previous.type) {
			decl.defaultValue = null;
		}
	}

	const TYPE_NAME: Record<RealmResourceKind, string> = {
		character: 'Character',
		lorebook: 'Lorebook',
		template: 'Context Template',
		persona: 'Persona',
		tool: 'Tool',
		connection_profile: 'Connection',
		widget: 'Widget'
	};

	const RUN_CONDITION_HINT =
		'Lets this agent take its turn only when this is true.\nLeave it empty and it always does.';

	const noun = (kind: RealmResourceKind) => TYPE_NAME[kind].toLowerCase();
</script>

<div class="inspector">
	{#if agent}
		{@const a = agent}
		<h3>Agent</h3>
		<label>
			<span>Handle</span>
			<input
				class="mono"
				class:candidate={textTakes('slug')}
				class:incompatible={incoming !== null && !textTakes('slug')}
				value={a.handle}
				spellcheck="false"
				oninput={(e) => (a.handle = e.currentTarget.value)}
				ondragover={(e) => textTakes('slug') && e.preventDefault()}
				ondrop={(e) => dropIntoText(e, 'slug', (next) => (a.handle = next))}
			/>
		</label>
		<p class="note">
			The agent's unique name in this realm. Scripts and other agents use it to find this one.
			<br />
			<br />
			{#if loopVar}
				<span class="warn"
					>You're in a for-each loop: every pass needs a different name, so build it from <code
						>&lbrace;&lbrace;{loopVar}&rbrace;&rbrace;</code
					>
					<br />
					<br />
					For example:
					<code>regent_&lbrace;&lbrace;{loopVar}.slug&rbrace;&rbrace;</code>.
				</span>
			{/if}
		</p>

		<label>
			<span>Name</span>
			<input
				class:candidate={textTakes('name')}
				class:incompatible={incoming !== null && !textTakes('name')}
				value={a.name}
				oninput={(e) => (a.name = e.currentTarget.value)}
				ondragover={(e) => textTakes('name') && e.preventDefault()}
				ondrop={(e) => dropIntoText(e, 'name', (next) => (a.name = next))}
			/>
		</label>

		<label>
			<span>Kind</span>
			<select value={a.kind} onchange={(e) => (a.kind = e.currentTarget.value as AgentKind)}>
				{#each AGENT_KINDS as kind (kind)}
					<option value={kind}>{kind}</option>
				{/each}
			</select>
		</label>

		{#if shows(a, 'character')}
			<div class="field">
				<span>Character</span>
				<RadioField
					value={a.character}
					kinds={['character']}
					{scope}
					{inputs}
					ariaLabel="Character"
					onlocate={locate}
					onchange={(next) => (a.character = next)}
				/>
			</div>
		{/if}

		{#if shows(a, 'template')}
			<div class="field">
				<span>Context template</span>
				<RadioField
					value={a.template}
					kinds={['template']}
					{scope}
					{inputs}
					ariaLabel="Context template"
					onlocate={locate}
					onchange={(next) => (a.template = next)}
				/>
			</div>
			<div class="field">
				<span>Connection</span>
				<RadioField
					value={a.profile}
					kinds={['connection_profile']}
					{scope}
					{inputs}
					ariaLabel="Connection profile"
					onlocate={locate}
					onchange={(next) => (a.profile = next)}
				/>
			</div>
		{/if}

		{#if shows(a, 'persona')}
			<div class="field">
				<span>Persona</span>
				<RadioField
					value={a.persona}
					kinds={['persona']}
					{scope}
					{inputs}
					ariaLabel="Persona"
					onlocate={locate}
					onchange={(next) => (a.persona = next)}
				/>
			</div>
		{/if}

		<div class="field">
			<span>Lorebooks</span>
			{#each a.lorebooks as book, i (i)}
				<RadioField
					value={book}
					kinds={['lorebook']}
					{scope}
					{inputs}
					ariaLabel="Lorebook {i + 1}"
					onlocate={locate}
					onchange={(next) => {
						if (next) a.lorebooks[i] = next;
						else a.lorebooks.splice(i, 1);
					}}
				/>
			{/each}
			<RadioField
				value={null}
				kinds={['lorebook']}
				{scope}
				{inputs}
				placeholder="Bind a lorebook…"
				ariaLabel="Bind another lorebook"
				onchange={(next) => next && a.lorebooks.push(next)}
			/>
		</div>

		<ContextField
			label="Run condition"
			value={a.run_condition}
			dialect="expression"
			multiline={false}
			tokens={false}
			maximizable={false}
			hint={RUN_CONDITION_HINT}
			onchange={(next) => (a.run_condition = next)}
		/>

		{#if shows(a, 'script')}
			<ContextField
				label="Script"
				value={a.script}
				dialect="template"
				rows={8}
				tokens={false}
				hint="Runs every time it's this agent's turn."
				onchange={(next) => (a.script = next)}
			/>
		{/if}
	{:else if loop?.kind === 'foreach'}
		{@const l = loop}
		{@const decl = realmDraft.inputs.find((i) => i.name === l.over)}
		<h3>For each</h3>
		<label>
			<span>Variable</span>
			<input
				class="mono"
				value={l.varName}
				spellcheck="false"
				oninput={(e) => (l.varName = e.currentTarget.value)}
			/>
		</label>
		<label>
			<span>Over</span>
			<select
				value={l.over ?? ''}
				onchange={(e) => {
					l.over = e.currentTarget.value || null;
					realmDraft.clearStaleBindings(l.key);
				}}
			>
				<option value="">— pick an input —</option>
				{#each realmDraft.inputs.filter((i) => i.ty.type === 'list') as i (i.name)}
					<option value={i.name}>{i.name} · {declaredText(i.ty)}</option>
				{/each}
			</select>
		</label>
		{#if decl}
			{@const yields = elementKind(decl.ty)}
			<p class="note">
				{#if yields}
					Repeats the agents below once for every {noun(yields)} in <code>{decl.name}</code>.
					Inside, <code>{l.varName}</code> is whichever one this pass is on.
				{:else}
					<code>{decl.name}</code> holds plain values rather than things you own. You can write
					<code>{l.varName}</code> into text, but it can't fill a character or lorebook field.
				{/if}
			</p>
		{:else}
			<p class="note">Pick a list input, or drag one onto the loop from the browser.</p>
		{/if}
	{:else if input}
		{@const decl = input}
		{@const base = baseOf(decl.ty)}
		{@const many = isList(decl.ty)}
		<h3>Input</h3>
		<p class="note">Something the person building a realm from this gets to choose.</p>

		<label>
			<span>Name</span>
			<input
				class="mono"
				value={decl.name}
				spellcheck="false"
				onchange={(e) => realmDraft.renameInput(decl.name, e.currentTarget.value)}
			/>
		</label>

		<label>
			<span>Holds</span>
			<select
				value={holdsValue(decl.ty)}
				onchange={(e) => retype(decl, holdsToBase(e.currentTarget.value), many)}
			>
				<optgroup label="Values">
					<option value="string">Text</option>
					<option value="number">A number</option>
					<option value="bool">Yes or no</option>
					<option value="enum">One of a set</option>
				</optgroup>
				<optgroup label="Something they already own">
					{#each RESOURCE_KINDS as kind (kind)}
						<option value="ref:{kind}">A {TYPE_NAME[kind]}</option>
					{/each}
				</optgroup>
			</select>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={many}
				onchange={(e) => retype(decl, base, e.currentTarget.checked)}
			/>
			<span>A list of them</span>
		</label>
		<p class="note">A list is what a <code>for each</code> in the cast repeats over.</p>

		{#if base.type === 'enum'}
			<label>
				<span>Choices</span>
				<input
					value={base.variants.join(', ')}
					placeholder="gentle, brutal"
					onchange={(e) => {
						const variants = e.currentTarget.value
							.split(',')
							.map((v) => v.trim())
							.filter(Boolean);
						realmDraft.retypeInput(decl.name, withList({ type: 'enum', variants }, many));
						if (decl.defaultValue !== null && !variants.includes(decl.defaultValue)) {
							decl.defaultValue = null;
						}
					}}
				/>
			</label>
		{/if}

		{#if base.type === 'reference'}
			<p class="note">
				They pick the {noun(base.kind ?? 'character')} themselves, so there's nothing sensible to fill
				in ahead of time. Always required.
			</p>
		{:else if many}
			<p class="note">Lists can't have a default yet, so this is always required.</p>
		{:else}
			<label>
				<span>Default</span>
				{#if base.type === 'enum'}
					<select
						value={decl.defaultValue ?? ''}
						onchange={(e) => (decl.defaultValue = e.currentTarget.value || null)}
					>
						<option value="">No default — required</option>
						{#each base.variants as variant (variant)}
							<option value={variant}>{variant}</option>
						{/each}
					</select>
				{:else if base.type === 'bool'}
					<select
						value={decl.defaultValue ?? ''}
						onchange={(e) => (decl.defaultValue = e.currentTarget.value || null)}
					>
						<option value="">No default — required</option>
						<option value="true">Yes</option>
						<option value="false">No</option>
					</select>
				{:else}
					<input
						value={decl.defaultValue ?? ''}
						type={base.type === 'number' ? 'number' : 'text'}
						placeholder="No default — required"
						onchange={(e) => (decl.defaultValue = e.currentTarget.value.trim() || null)}
					/>
				{/if}
			</label>
		{/if}

		<div class="declaration">
			<span class="declLabel">Declares</span>
			<code>{declarationLine(decl)}</code>
			<span class="declTag" class:optional={!isRequired(decl)}>
				{isRequired(decl) ? 'required' : 'optional'}
			</span>
		</div>

		<button type="button" class="danger" onclick={() => realmDraft.removeInput(decl.name)}>
			<Trash2 size={12} /> Delete input
		</button>
	{:else if resource}
		<h3>{TYPE_NAME[resource.resourceKind]}</h3>
		<p class="resourceName">{resource.name}</p>
		<p class="note"><code>{resource.id}</code></p>
		<button type="button" onclick={() => openResource(resource.resourceKind, resource.id)}>
			<ExternalLink size={12} /> Open in Studio
		</button>
	{:else}
		<p class="nothing">Select something in the cast, or an input in the browser.</p>
	{/if}
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		min-width: 0;
	}
	h3 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
		color: var(--color-text-primary);
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
	.mono {
		font-family: var(--font-family-mono);
	}
	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
	}
	.note code {
		font-family: var(--font-family-mono);
		color: var(--color-text-secondary);
	}
	.danger {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background: transparent;
		border: 1px solid var(--color-error-border);
		border-radius: var(--radius-sm);
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
	}
	.danger:hover {
		background-color: var(--color-error-bg);
	}
	/* The same drag vocabulary the radio fields use. */
	.candidate {
		border-color: var(--color-border-active) !important;
		border-style: dashed !important;
	}
	.incompatible {
		opacity: 0.35;
		filter: grayscale(1);
	}

	/* A checkbox reads as one line, not as a label stacked over a control. */
	.check {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.check input {
		flex: 0 0 auto;
		margin: 0;
	}
	.check > span {
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
	}

	.declaration {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: var(--spacing-xs) var(--spacing-sm);
		padding: var(--spacing-sm);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.declLabel {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	.declaration code {
		flex: 1 1 100%;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--color-text-accent);
		overflow-wrap: anywhere;
	}
	.declTag {
		padding: 1px 6px;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.declTag.optional {
		color: var(--color-text-muted);
	}

	.resourceName {
		margin: 0;
		color: var(--color-text-primary);
		font-size: var(--font-size-md);
	}
	.inspector > button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.inspector > button:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.nothing {
		margin: var(--spacing-lg) 0;
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.warn {
		color: var(--color-warning-text);
	}
</style>
