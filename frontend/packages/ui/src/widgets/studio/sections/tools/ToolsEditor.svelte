<script lang="ts">
	import { onDestroy } from 'svelte';
	import { navigator } from '$lib/navigation/navigator.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type {
		Param,
		ParamType,
		PickableResource,
		ToolSurface,
		UserTool
	} from '$lib/types/tools.js';
	import { RESOURCE_KINDS, resourceLabel } from '$lib/commands/index.js';
	import { builtInSlashCommands } from '$lib/commands/index.js';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import FieldShell from '$lib/components/FieldShell.svelte';
	import ContextField from '$lib/components/ContextField.svelte';
	import ListSidebar from '$lib/components/ListSidebar.svelte';

	const SAVE_MS = 500;
	const MAX_DEPTH = 3; // UI cap on array nesting

	type Base = 'string' | 'number' | 'bool' | 'object';

	let { selectedId }: { selectedId: Snowflake | null } = $props();

	const draft = $derived(toolStore.editingTool);
	const save = $derived(toolStore.saveState);
	const tools = $derived(toolStore.loadedTools ?? []);
	const builtins = $derived(toolStore.builtinTools ?? []);
	const shown = $derived(draft ?? toolStore.viewingTool);
	const readOnly = $derived(!draft);

	let query = $state('');
	let sort = $state('name-asc');

	function arrange(list: UserTool[]): UserTool[] {
		const needle = query.trim().toLowerCase();
		const out = list.filter(
			(t) =>
				needle === '' ||
				t.name.toLowerCase().includes(needle) ||
				(t.display_name ?? '').toLowerCase().includes(needle)
		);
		if (sort === 'name-desc') return out.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'enabled') return out.sort((a, b) => Number(b.enabled) - Number(a.enabled));
		return out.sort((a, b) => a.name.localeCompare(b.name));
	}

	const shownTools = $derived(arrange(tools));
	const shownBuiltins = $derived(arrange(builtins));
	const noMatches = $derived(shownTools.length === 0 && shownBuiltins.length === 0);

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		if (readOnly) return;
		clearTimeout(timer);
		timer = setTimeout(() => void toolStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function flushPending() {
		clearTimeout(timer);
		await toolStore.save();
	}

	async function select(id: Snowflake) {
		if (id === selectedId) return;
		await flushPending();
		navigator.start().studio().tools(id);
	}

	async function newTool() {
		await flushPending();
		const id = await toolStore.createTool();
		if (id) navigator.start().studio().tools(id);
	}

	async function removeTool(id: Snowflake, name: string) {
		if (!confirm(`Delete "${name || 'Untitled'}"? This can't be undone.`)) return;
		clearTimeout(timer);
		await toolStore.deleteTool(id);
		if (id === selectedId) navigator.start().studio().tools();
	}

	function flatten(ty: ParamType): { base: Base; depth: number } {
		let depth = 0;
		let cur: ParamType = ty;
		while (typeof cur === 'object') {
			depth++;
			cur = cur.array;
		}
		return { base: cur, depth };
	}
	function buildType(base: Base, depth: number): ParamType {
		let ty: ParamType = base;
		for (let i = 0; i < depth; i++) ty = { array: ty };
		return ty;
	}

	/** Whether `resource` can apply: the server rejects it on anything else. */
	function nameable(p: Param): boolean {
		const flat = flatten(p.ty);
		return flat.base === 'string' && flat.depth === 0;
	}

	function setBase(p: Param, base: Base) {
		p.ty = buildType(base, flatten(p.ty).depth);
		if (!nameable(p)) p.resource = undefined;
		requestSave();
	}
	function setDepth(p: Param, depth: number) {
		const clamped = Math.max(0, Math.min(MAX_DEPTH, depth));
		p.ty = buildType(flatten(p.ty).base, clamped);
		if (!nameable(p)) p.resource = undefined;
		requestSave();
	}
	function setResource(p: Param, kind: string) {
		p.resource = kind === '' ? undefined : (kind as PickableResource);
		requestSave();
	}
	function addParam() {
		draft?.params.push({ name: '', ty: 'string', description: null, required: true });
		requestSave();
	}
	function removeParam(i: number) {
		draft?.params.splice(i, 1);
		requestSave();
	}

	const CLIENT_COMMAND_NAMES = new Set(builtInSlashCommands.map((c) => c.name));

	/** Every name a command of your own would take over: the client built-ins
	 *  and the shipped command rows. */
	const overridable = $derived(
		new Set([
			...CLIENT_COMMAND_NAMES,
			...builtins.filter((t) => t.surface !== 'tool').map((t) => t.name)
		])
	);

	/** A command taking one of those names replaces it, which is supported but
	 *  worth saying out loud before someone loses /newchat by accident. */
	const shadowed = $derived(
		draft && draft.surface !== 'tool' && overridable.has(draft.name) && draft.name !== 'send'
			? draft.name
			: null
	);

	/** A shipped row you have not already taken over. */
	const forkable = $derived(
		readOnly && shown && !tools.some((t) => t.name === shown.name) ? shown : null
	);

	async function forkBuiltin() {
		if (!forkable) return;
		const id = await toolStore.forkBuiltin(forkable.id);
		if (id) navigator.start().studio().tools(id);
	}

	const saveLabel = $derived(
		save.status === 'saving'
			? 'Saving…'
			: save.status === 'error'
				? (save.message ?? 'Error')
				: toolStore.hasUnsavedChanges
					? 'Unsaved'
					: 'Saved'
	);
</script>

{#snippet hint(text: string)}
	<button type="button" class="hint" title={text} aria-label={`Help: ${text}`}>?</button>
{/snippet}

<div class="tool-editor">
	<ListSidebar
		title="Tools"
		oncreate={newTool}
		bind:query
		bind:sort
		sortOptions={[
			{ value: 'name-asc', label: 'Name (A–Z)' },
			{ value: 'name-desc', label: 'Name (Z–A)' },
			{ value: 'enabled', label: 'Enabled first' }
		]}
		searchPlaceholder="Search tools…"
	>
		{#snippet status()}
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span class="save-label">{saveLabel}</span>
				</div>
			{/if}
		{/snippet}

		{#if noMatches}
			<p class="empty">{tools.length === 0 ? 'No tools yet.' : 'No tools match that search.'}</p>
		{/if}
		{#each shownTools as tool (tool.id)}
			<div class="tool-item" class:selected={tool.id === selectedId} class:disabled={!tool.enabled}>
				<button class="tool-open" type="button" onclick={() => select(tool.id)}>
					<span class="tool-text">
						<span class="tool-name">{tool.display_name || tool.name}</span>
						{#if tool.display_name}
							<span class="tool-sub">{tool.name}</span>
						{/if}
					</span>
				</button>
				<input
					type="checkbox"
					class="switch enable-toggle"
					checked={tool.enabled}
					onchange={() => toolStore.setEnabled(tool.id, !tool.enabled)}
					title={tool.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'}
					aria-label={tool.enabled ? `Disable ${tool.name}` : `Enable ${tool.name}`}
				/>
			</div>
		{/each}

		{#if shownBuiltins.length > 0}
			<p class="group-label">Built-in</p>
			{#each shownBuiltins as tool (tool.id)}
				<div class="tool-item" class:selected={tool.id === selectedId}>
					<button class="tool-open" type="button" onclick={() => select(tool.id)}>
						<span class="tool-text">
							<span class="tool-name">{tool.name}</span>
							{#if tool.display_name}
								<span class="tool-sub">{tool.display_name}</span>
							{/if}
						</span>
					</button>
				</div>
			{/each}
		{/if}
	</ListSidebar>

	<section class="col-content">
		{#if shown}
			<div class="pane">
				<div class="pane-head">
					<h1>{readOnly ? 'Built-in tool' : 'Edit tool'}</h1>
					{#if !readOnly}
						<button type="button" class="delete" onclick={() => removeTool(shown.id, shown.name)}>
							Delete
						</button>
					{/if}
				</div>

				{#if readOnly}
					<div class="readonly-note">
						<p>
							Ships with the server and is available to every realm without an import. Read-only —
							make a copy to change how it behaves.
						</p>
						{#if forkable}
							<button type="button" class="fork" onclick={forkBuiltin}> Create my version </button>
						{:else if shown}
							<p class="already-forked">
								You already have a <code>{shown.name}</code> of your own, which takes precedence over
								this one.
							</p>
						{/if}
					</div>
				{/if}

				<div class="field">
					<label for="t-name">
						Name {@render hint(
							'The identifier the model calls. Letters, digits, _ or -, up to 64.'
						)}
					</label>
					<input
						id="t-name"
						class="mono"
						readonly={readOnly}
						bind:value={shown.name}
						oninput={requestSave}
					/>
				</div>

				<div class="field">
					<label for="t-display">
						Display name {@render hint('Shown in the UI instead of the name. Optional.')}
					</label>
					<input
						id="t-display"
						placeholder={shown.name}
						readonly={readOnly}
						bind:value={shown.display_name}
						oninput={requestSave}
					/>
				</div>

				<div class="field">
					<label for="t-surface">
						Callable from {@render hint(
							'A tool is offered to the model on a turn.\nA command is invoked by you as /name in the composer.\nThe same body can serve both.'
						)}
					</label>
					<select
						id="t-surface"
						disabled={readOnly}
						value={shown.surface}
						onchange={(e) => {
							shown.surface = e.currentTarget.value as ToolSurface;
							requestSave();
						}}
					>
						<option value="tool">The model, as a tool</option>
						<option value="command">You, as /{shown.name || 'name'}</option>
						<option value="both">Both</option>
					</select>
				</div>

				{#if shadowed}
					<p class="shadow-note">
						Overrides the built-in <code>/{shadowed}</code>. Delete or rename this command to get
						the original back.
					</p>
				{/if}

				{#key shown.id}
					<ContextField
						label="Description"
						hint="Tells the model when and why to call this tool."
						value={shown.description}
						rows={3}
						{readOnly}
						weaver={false}
						onchange={(v) => {
							shown.description = v;
							requestSave();
						}}
					/>
				{/key}

				<div class="field">
					<div class="params-head">
						<span class="params-title">
							Parameters {@render hint('Become available in the cmd namespace, e.g. {{cmd:name}}.')}
						</span>
						{#if !readOnly}
							<button type="button" class="add-param" onclick={addParam}>Add parameter</button>
						{/if}
					</div>

					{#if shown.params.length === 0}
						<p class="empty params-empty">No parameters. The tool takes no input.</p>
					{/if}

					<div class="param-cards">
						{#each shown.params as param, i (i)}
							{@const flat = flatten(param.ty)}
							<div class="param-card">
								<div class="param-card-top">
									<input
										class="param-name mono"
										placeholder="name"
										readonly={readOnly}
										bind:value={param.name}
										oninput={requestSave}
									/>
									<select
										class="param-type"
										value={flat.base}
										disabled={readOnly}
										onchange={(e) => setBase(param, e.currentTarget.value as Base)}
									>
										<option value="string">string</option>
										<option value="number">number</option>
										<option value="bool">bool</option>
										<option value="object">object</option>
									</select>
									{#if nameable(param)}
										<select
											class="param-resource"
											title="Offer a search for this kind and store its id"
											value={param.resource ?? ''}
											disabled={readOnly}
											onchange={(e) => setResource(param, e.currentTarget.value)}
										>
											<option value="">free text</option>
											{#each RESOURCE_KINDS as kind (kind)}
												<option value={kind}>{resourceLabel(kind)}</option>
											{/each}
										</select>
									{/if}
									<div class="depth" title="Array nesting">
										{#if !readOnly}
											<button
												type="button"
												onclick={() => setDepth(param, flat.depth - 1)}
												disabled={flat.depth === 0}
												aria-label="Remove array layer">−</button
											>
										{/if}
										<span class="depth-label mono">{'[]'.repeat(flat.depth) || '·'}</span>
										{#if !readOnly}
											<button
												type="button"
												onclick={() => setDepth(param, flat.depth + 1)}
												disabled={flat.depth >= MAX_DEPTH}
												aria-label="Add array layer">+</button
											>
										{/if}
									</div>
								</div>

								<input
									class="param-desc"
									placeholder="description (optional)"
									readonly={readOnly}
									bind:value={param.description}
									oninput={requestSave}
								/>

								<div class="param-card-foot">
									<label class="param-required">
										<input
											type="checkbox"
											disabled={readOnly}
											bind:checked={param.required}
											onchange={requestSave}
										/>
										<span>Required</span>
									</label>
									{#if !readOnly}
										<button type="button" class="param-remove" onclick={() => removeParam(i)}>
											Remove
										</button>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>

				<FieldShell
					label="Body"
					hint="weaver-lang source. Its output is returned to the model as the tool result."
				>
					{#snippet editor(fill: boolean)}
						<div class="body-editor" class:fill>
							{#key shown.id}
								<WeaverEditor
									doc={shown.body}
									{readOnly}
									onChange={readOnly
										? undefined
										: (v) => {
												shown.body = v;
												requestSave();
											}}
								/>
							{/key}
						</div>
					{/snippet}
				</FieldShell>
			</div>
		{:else}
			<div class="pane empty-pane">
				<p class="empty">Select a tool to edit, or create one.</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.tool-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
		position: relative;
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
		overflow-y: auto;
		position: relative;
	}
	.save-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		min-width: 0;
	}
	.save-row.error {
		color: var(--color-error-text);
	}
	.save-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.dot {
		flex: 0 0 auto;
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background-color: var(--color-text-muted);
	}
	.dot[data-status='saving'] {
		background-color: var(--color-warning);
	}
	.dot[data-status='saved'] {
		background-color: var(--color-success);
	}
	.dot[data-status='error'] {
		background-color: var(--color-error);
	}
	.group-label {
		margin: var(--spacing-md) 0 var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
	}
	.tool-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-sm);
	}
	.tool-item:hover {
		background-color: var(--color-hover);
	}
	.tool-item.selected {
		background-color: var(--color-selected);
	}
	.tool-item.disabled .tool-name {
		color: var(--color-text-muted);
	}
	.tool-open {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		background: transparent;
		padding: var(--spacing-sm);
		text-align: left;
	}
	.tool-text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.tool-name {
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tool-sub {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.enable-toggle {
		flex: 0 0 auto;
		width: 32px;
		height: 18px;
		padding: 2px;
		display: inline-flex;
		align-items: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		transition: background-color 0.15s ease;
	}
	.pane {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
		padding: var(--spacing-xl);
	}
	.pane-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.empty-pane {
		align-items: center;
		justify-content: center;
	}
	.pane h1 {
		font-size: var(--font-size-h3);
		margin: 0;
	}
	.delete {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
		color: var(--color-error-text);
	}
	.readonly-note p {
		margin: 0;
	}
	.readonly-note {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--spacing-sm);
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.field {
		display: flex;
		flex-direction: column;
	}
	label {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin-top: var(--spacing-xs);
	}
	.mono {
		font-family: var(--font-family-mono);
	}
	.body-editor {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.body-editor.fill {
		flex: 1 1 auto;
		height: 100%;
	}
	.body-editor.fill > :global(*) {
		flex: 1 1 auto;
		min-height: 0;
	}

	.params-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: var(--spacing-xs);
		margin-bottom: var(--spacing-sm);
	}
	.params-title {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.add-param {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.params-empty {
		margin: 0 0 var(--spacing-sm);
	}
	.param-cards {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}
	.param-card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}
	.param-card-top {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.param-name {
		flex: 1 1 auto;
		width: auto;
		min-width: 0;
	}
	.param-type {
		flex: 0 0 auto;
		width: auto;
		min-width: 7rem;
	}
	.fork {
		width: auto;
	}
	.already-forked {
		color: var(--color-text-muted);
	}
	.shadow-note {
		margin: 0;
		padding: var(--spacing-sm);
		border-left: 2px solid var(--color-warning-border);
		background-color: var(--color-warning-bg);
		color: var(--color-warning-text);
		border-radius: var(--radius-sm, 6px);
		font-size: var(--font-size-sm, 0.85rem);
	}
	.param-resource {
		flex: 0 0 auto;
		width: auto;
		min-width: 9rem;
	}
	.depth {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.depth button {
		width: 22px;
		height: 22px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		line-height: 1;
	}
	.depth button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.depth-label {
		min-width: 28px;
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.param-card-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.param-required {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.param-remove {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.param-remove:hover {
		color: var(--color-error-text);
	}
	.hint {
		flex: 0 0 auto;
		width: 16px;
		height: 16px;
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		line-height: 1;
		cursor: help;
	}
	.hint:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
