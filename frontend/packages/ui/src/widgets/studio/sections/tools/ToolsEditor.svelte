<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { Param, ParamType } from '$lib/types/tools.js';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';
	import FieldShell from '$lib/components/FieldShell.svelte';
	import ContextField from '$lib/components/ContextField.svelte';

	const SAVE_MS = 500;
	const MAX_DEPTH = 3; // UI cap on array nesting

	type Base = 'string' | 'number' | 'bool' | 'object';

	const draft = $derived(toolStore.editingTool);
	const save = $derived(toolStore.saveState);
	const tools = $derived(toolStore.loadedTools ?? []);
	const builtins = $derived(toolStore.builtinTools ?? []);
	const selectedId = $derived(toolStore.selectedId);
	const shown = $derived(draft ?? toolStore.viewingTool);
	const readOnly = $derived(!draft);

	onMount(() => {
		void toolStore.loadTools();
		void toolStore.loadBuiltins();
	});
	onDestroy(() => toolStore.stopEditing());

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		if (readOnly) return;
		clearTimeout(timer);
		timer = setTimeout(() => void toolStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function select(id: Snowflake) {
		if (id === toolStore.editingId) return;
		clearTimeout(timer);
		await toolStore.save();
		void toolStore.startEditing(id);
	}

	async function selectBuiltin(id: Snowflake) {
		if (id === toolStore.selectedId) return;
		clearTimeout(timer);
		await toolStore.save();
		void toolStore.viewBuiltin(id);
	}

	async function newTool() {
		clearTimeout(timer);
		await toolStore.save();
		void toolStore.createTool();
	}

	async function removeTool(id: Snowflake, name: string) {
		if (!confirm(`Delete "${name || 'Untitled'}"? This can't be undone.`)) return;
		await toolStore.deleteTool(id);
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

	function setBase(p: Param, base: Base) {
		p.ty = buildType(base, flatten(p.ty).depth);
		requestSave();
	}
	function setDepth(p: Param, depth: number) {
		const clamped = Math.max(0, Math.min(MAX_DEPTH, depth));
		p.ty = buildType(flatten(p.ty).base, clamped);
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
	<section class="col-list">
		<header class="list-header">
			<div class="header-row">
				<span class="title">Tools</span>
				<button type="button" class="new-tool" onclick={newTool}>New</button>
			</div>
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span class="save-label">{saveLabel}</span>
				</div>
			{/if}
		</header>

		<nav class="tool-list">
			{#if tools.length === 0}
				<p class="empty">No tools yet.</p>
			{/if}
			{#each tools as tool (tool.id)}
				<div
					class="tool-item"
					class:selected={tool.id === selectedId}
					class:disabled={!tool.enabled}
				>
					<button class="tool-open" type="button" onclick={() => select(tool.id)}>
						<span class="tool-text">
							<span class="tool-name">{tool.name || 'Untitled'}</span>
							{#if tool.display_name}
								<span class="tool-sub">{tool.display_name}</span>
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

			{#if builtins.length > 0}
				<p class="group-label">Built-in</p>
				{#each builtins as tool (tool.id)}
					<div class="tool-item" class:selected={tool.id === selectedId}>
						<button class="tool-open" type="button" onclick={() => selectBuiltin(tool.id)}>
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
		</nav>
	</section>

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
					<p class="readonly-note">
						Ships with the server and is available to every realm without an import. Read-only —
						copy it into a tool of your own to change how it behaves.
					</p>
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
	.col-list {
		flex: 0 0 240px;
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		border-right: 1px solid var(--color-border-subtle);
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
		overflow-y: auto;
		position: relative;
	}
	.list-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.header-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.title {
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.new-tool {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
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
	.tool-list {
		display: flex;
		flex-direction: column;
		padding: var(--spacing-sm);
		gap: var(--spacing-xs);
		overflow-y: auto;
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
	.readonly-note {
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
