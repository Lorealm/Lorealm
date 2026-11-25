<script lang="ts">
	import ConfirmDelete from '#lib/components/ConfirmDelete.svelte';
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onDestroy } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { toolStore } from '#lib/stores/tools.svelte.js';
	import { exportItem } from '#lib/stores/transfer.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type {
		Param,
		ParamType,
		PickableResource,
		ToolSurface,
		UserTool
	} from '#lib/types/tools.js';
	import { RESOURCE_KINDS, resourceLabel } from '#lib/commands/index.js';
	import { builtInSlashCommands } from '#lib/commands/index.js';
	import WeaverEditor from '#lib/weaver/editor/WeaverEditor.svelte';
	import FieldShell from '#lib/components/FieldShell.svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import ListSidebar from '#lib/components/ListSidebar.svelte';
	import Badge from '#lib/widgets/realms/Badge.svelte';

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
	let showBuiltins = $state(true);

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

	const builtinIds = $derived(new Set(builtins.map((t) => t.id)));
	const shownTools = $derived(arrange([...tools, ...(showBuiltins ? builtins : [])]));

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

	async function removeTool(id: Snowflake) {
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

	const overridable = $derived(
		new Set([
			...CLIENT_COMMAND_NAMES,
			...builtins.filter((t) => t.surface !== 'tool').map((t) => t.name)
		])
	);

	const shadowed = $derived(
		draft && draft.surface !== 'tool' && overridable.has(draft.name) && draft.name !== 'send'
			? draft.name
			: null
	);

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
			? t.common.saving
			: save.status === 'error'
				? (save.message ?? t.common.error)
				: toolStore.hasUnsavedChanges
					? t.studio.tools.unsaved
					: t.common.saved
	);
</script>

{#snippet hint(text: string)}
	<button
		type="button"
		class="hint"
		{@attach tooltip(text)}
		aria-label={t.components.field.help({ label: text })}>?</button
	>
{/snippet}

<div class="tool-editor">
	<ListSidebar
		title={t.studio.tools.title}
		detailOpen={selectedId !== null}
		oncreate={newTool}
		bind:query
		bind:sort
		bind:showBuiltins
		sortOptions={[
			{ value: 'name-asc', label: t.studio.tools.sortNameAsc },
			{ value: 'name-desc', label: t.studio.tools.sortNameDesc },
			{ value: 'enabled', label: t.studio.tools.sortEnabled }
		]}
		searchPlaceholder={t.studio.tools.search}
	>
		{#snippet status()}
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span class="save-label">{saveLabel}</span>
				</div>
			{/if}
		{/snippet}

		{#if shownTools.length === 0}
			<p class="empty">{tools.length === 0 ? t.studio.tools.empty : t.studio.tools.noMatches}</p>
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
				{#if builtinIds.has(tool.id)}
					<Badge>{t.studio.tools.builtIn}</Badge>
				{:else}
					<input
						type="checkbox"
						class="switch enable-toggle"
						checked={tool.enabled}
						onchange={() => toolStore.setEnabled(tool.id, !tool.enabled)}
						{@attach tooltip(
							tool.enabled ? t.studio.tools.enabledHint : t.studio.tools.disabledHint
						)}
						aria-label={tool.enabled
							? t.studio.tools.disableNamed({ name: tool.name })
							: t.studio.tools.enableNamed({ name: tool.name })}
					/>
				{/if}
			</div>
		{/each}
	</ListSidebar>

	<section class="col-content">
		{#if shown}
			<div class="pane">
				<div class="pane-head">
					<h1>{readOnly ? t.studio.tools.builtInTool : t.studio.tools.editTool}</h1>
					{#if !readOnly}
						<button
							type="button"
							class="export"
							onclick={() => exportItem('tool', shown.id, shown.name)}
						>
							{t.studio.tools.export}
						</button>
						<ConfirmDelete
							prompt={t.studio.tools.confirmDelete({ name: shown.name || t.studio.tools.untitled })}
							onConfirm={() => removeTool(shown.id)}
						>
							{#snippet trigger(arm)}
								<button type="button" class="delete" onclick={arm}>
									{t.common.delete}
								</button>
							{/snippet}
						</ConfirmDelete>
					{/if}
				</div>

				{#if readOnly}
					<div class="readonly-note">
						<p>
							{t.studio.tools.builtInNote}
						</p>
						{#if forkable}
							<button type="button" class="fork" onclick={forkBuiltin}>{t.studio.tools.fork}</button
							>
						{:else if shown}
							<p class="already-forked">
								{t.studio.tools.alreadyForkedPrefix} <code>{shown.name}</code>
								{t.studio.tools.alreadyForkedSuffix}
							</p>
						{/if}
					</div>
				{/if}

				<div class="field">
					<label for="t-name">
						{t.studio.tools.name}
						{@render hint(t.studio.tools.nameHint)}
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
						{t.studio.tools.displayName}
						{@render hint(t.studio.tools.displayNameHint)}
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
						{t.studio.tools.callableFrom}
						{@render hint(t.studio.tools.callableHint)}
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
						<option value="tool">{t.studio.tools.surfaceTool}</option>
						<option value="command"
							>{t.studio.tools.surfaceCommand({
								name: shown.name || t.studio.tools.namePlaceholder
							})}</option
						>
						<option value="both">{t.studio.tools.surfaceBoth}</option>
					</select>
				</div>

				{#if shadowed}
					<p class="shadow-note">
						{t.studio.tools.overridesPrefix} <code>/{shadowed}</code>.
						{t.studio.tools.overridesSuffix}
					</p>
				{/if}

				{#key shown.id}
					<ContextField
						label={t.studio.tools.description}
						hint={t.studio.tools.descriptionHint}
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
							{t.studio.tools.parameters}
							{@render hint(t.studio.tools.parametersHint)}
						</span>
						{#if !readOnly}
							<button type="button" class="add-param" onclick={addParam}
								>{t.studio.tools.addParameter}</button
							>
						{/if}
					</div>

					{#if shown.params.length === 0}
						<p class="empty params-empty">{t.studio.tools.noParameters}</p>
					{/if}

					<div class="param-cards">
						{#each shown.params as param, i (i)}
							{@const flat = flatten(param.ty)}
							<div class="param-card">
								<div class="param-card-top">
									<input
										class="param-name mono"
										placeholder={t.studio.tools.namePlaceholder}
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
											{@attach tooltip(t.studio.tools.resourceHint)}
											value={param.resource ?? ''}
											disabled={readOnly}
											onchange={(e) => setResource(param, e.currentTarget.value)}
										>
											<option value="">{t.studio.tools.freeText}</option>
											{#each RESOURCE_KINDS as kind (kind)}
												<option value={kind}>{resourceLabel(kind)}</option>
											{/each}
										</select>
									{/if}
									<div class="depth" {@attach tooltip(t.studio.tools.arrayNesting)}>
										{#if !readOnly}
											<button
												type="button"
												onclick={() => setDepth(param, flat.depth - 1)}
												disabled={flat.depth === 0}
												aria-label={t.studio.tools.removeLayer}>−</button
											>
										{/if}
										<span class="depth-label mono">{'[]'.repeat(flat.depth) || '·'}</span>
										{#if !readOnly}
											<button
												type="button"
												onclick={() => setDepth(param, flat.depth + 1)}
												disabled={flat.depth >= MAX_DEPTH}
												aria-label={t.studio.tools.addLayer}>+</button
											>
										{/if}
									</div>
								</div>

								<input
									class="param-desc"
									placeholder={t.studio.tools.paramDescription}
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
										<span>{t.studio.tools.required}</span>
									</label>
									{#if !readOnly}
										<button type="button" class="param-remove" onclick={() => removeParam(i)}>
											{t.studio.tools.remove}
										</button>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				</div>

				<FieldShell label={t.studio.tools.body} hint={t.studio.tools.bodyHint}>
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
				<p class="empty">{t.studio.tools.select}</p>
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
	.export {
		flex: 0 0 auto;
		margin-left: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
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

	@media (max-width: 768px) {
		.pane {
			padding: var(--spacing-md);
		}
	}
</style>
