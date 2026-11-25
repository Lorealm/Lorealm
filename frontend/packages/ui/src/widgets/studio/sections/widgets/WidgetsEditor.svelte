<script lang="ts">
	import ConfirmDelete from '#lib/components/ConfirmDelete.svelte';
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { navigator } from '#lib/navigation/navigator.js';
	import { toolStore } from '#lib/stores/tools.svelte.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import { exportItem } from '#lib/stores/transfer.js';
	import { usePlatform } from '#lib/api/client.js';
	import { showError } from '#lib/stores/errorToast.svelte.js';
	import type { JsonValue, Snowflake } from '#lib/types/core.js';
	import type { WidgetMode, WidgetPlacement } from '#lib/types/widgets.js';
	import { onDestroy, untrack } from 'svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import StateFrame from '#lib/components/StateFrame.svelte';
	import HtmlEditor from '#lib/components/HtmlEditor.svelte';
	import ListSidebar from '#lib/components/ListSidebar.svelte';
	import TagField from '../characters/TagField.svelte';
	import Badge from '#lib/widgets/realms/Badge.svelte';
	import { LayoutDashboard, Paperclip } from '@lucide/svelte';

	const SAVE_MS = 500;

	/** Skeleton for a new widget: reads the state slice the host posts in. */
	const STARTER = `<!doctype html>
<meta charset="utf-8" />
<style>
  body { margin: 0; font: 13px system-ui, sans-serif; color: var(--color-text-primary); background: transparent; }
  pre { margin: 0; padding: 8px; white-space: pre-wrap; word-break: break-word; }
</style>
<pre id="out">waiting for state…</pre>
<script>
  // The host posts { type: 'lorealm:state', state } on every committed turn.
  // (It also handles sizing this iframe to your content automatically.)
  addEventListener('message', (e) => {
    if (e.data?.type !== 'lorealm:state') return;
    document.getElementById('out').textContent = JSON.stringify(e.data.state, null, 2);
  });

  // parent.postMessage({ type: 'lorealm:run', source: '$[set_var("world:x", 1)]', label: 'Set x' }, '*');
  // parent.postMessage({ type: 'lorealm:action', tool_id: '…', args: {} }, '*');
${'</'}script>
`;

	let { selectedId }: { selectedId: Snowflake | null } = $props();

	const widgets = $derived(userWidgetStore.loadedWidgets ?? []);
	const builtins = $derived(userWidgetStore.builtinWidgets ?? []);
	const builtinIds = $derived(new Set(builtins.map((w) => w.id)));
	const selected = $derived(
		widgets.find((w) => w.id === selectedId) ?? builtins.find((w) => w.id === selectedId) ?? null
	);
	const readOnly = $derived(selected !== null && builtinIds.has(selected.id));

	const forkable = $derived(
		readOnly && selected && !widgets.some((w) => w.name === selected.name) ? selected : null
	);

	let query = $state('');
	let sort = $state('name-asc');
	let showBuiltins = $state(true);
	let creating = $state(false);

	const shownWidgets = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const out = [...widgets, ...(showBuiltins ? builtins : [])].filter(
			(w) =>
				needle === '' ||
				w.name.toLowerCase().includes(needle) ||
				w.description.toLowerCase().includes(needle)
		);
		if (sort === 'name-desc') return out.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'enabled') return out.sort((a, b) => Number(b.enabled) - Number(a.enabled));
		return out.sort((a, b) => a.name.localeCompare(b.name));
	});

	const availableTools = $derived([
		...(toolStore.loadedTools ?? []).map((t) => ({ id: t.id, name: t.display_name ?? t.name })),
		...(toolStore.builtinTools ?? []).map((tool) => ({
			id: tool.id,
			name: t.studio.widgets.builtinSuffix({ name: tool.display_name ?? tool.name })
		}))
	]);
	const toolName = (id: Snowflake) => availableTools.find((t) => t.id === id)?.name ?? id;

	// Local draft
	let draftName = $state('');
	let draftDescription = $state('');
	let draftSource = $state('');
	let draftPrompt = $state('');
	let draftRender = $state('');
	let draftTools = $state<Snowflake[]>([]);
	let draftPlacement = $state<WidgetPlacement>('widget');
	let draftMode = $state<WidgetMode>('latest');
	let draftProps = $state<string[]>([]);
	let toolPick = $state('');
	let loadedId: string | null = null;
	let syncedId = $state<string | null>(null);

	const isAttachment = $derived(draftPlacement === 'attachment');

	const TABS = ['source', 'model', 'settings'] as const;
	let tab = $state<(typeof TABS)[number]>('source');
	let payloadOpen = $state(false);

	$effect(() => {
		if (selected && selected.id !== loadedId) {
			loadedId = selected.id;
			draftName = selected.name;
			draftDescription = selected.description;
			draftSource = selected.source;
			draftPrompt = selected.prompt;
			draftRender = selected.render;
			draftTools = [...selected.tools];
			draftPlacement = selected.placement;
			draftMode = selected.mode;
			draftProps = [...selected.props];
			untrack(() => void loadPayload());
			syncedId = selected.id;
		} else if (!selected) {
			loadedId = null;
			syncedId = null;
		}
	});

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void saveNow(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function saveNow() {
		const id = loadedId;
		if (!id || builtinIds.has(id)) return;
		await userWidgetStore.updateWidget(id, {
			name: draftName,
			description: draftDescription,
			source: draftSource,
			prompt: draftPrompt,
			render: draftRender,
			tools: draftTools,
			placement: draftPlacement,
			mode: draftMode,
			props: draftProps.map((p) => p.trim()).filter(Boolean)
		});
	}

	function bindTool() {
		if (!toolPick || draftTools.includes(toolPick)) return;
		draftTools = [...draftTools, toolPick];
		toolPick = '';
		requestSave();
	}

	function unbindTool(id: Snowflake) {
		draftTools = draftTools.filter((t) => t !== id);
		requestSave();
	}

	async function select(id: Snowflake) {
		if (id === selectedId) return;
		clearTimeout(timer);
		await saveNow();
		navigator.start().studio().widgets(id);
	}

	async function newWidget() {
		if (creating) return;
		creating = true;
		clearTimeout(timer);
		try {
			await saveNow();
			const created = await userWidgetStore.createWidget({
				name: t.studio.widgets.newName,
				source: STARTER
			});
			if (created) navigator.start().studio().widgets(created.id);
		} finally {
			creating = false;
		}
	}

	async function forkBuiltin() {
		if (!forkable) return;
		const { name, description, source, prompt, render, tools, placement, mode, props } = forkable;
		const created = await userWidgetStore.createWidget({
			name,
			description,
			source,
			prompt,
			render,
			tools,
			placement,
			mode,
			props
		});
		if (created) navigator.start().studio().widgets(created.id);
	}

	async function removeWidget(id: Snowflake) {
		clearTimeout(timer);
		await userWidgetStore.deleteWidget(id);
		for (const placement of ['widget', 'attachment'] as const) {
			void usePlatform().store.delete(payloadKey(id, placement));
		}
		if (selectedId === id) navigator.start().studio().widgets();
	}

	const SAMPLE = {
		world: { season: 'winter' },
		self: { stamina: 3, inventory: { sword: 1, potion: 2 } }
	};

	let payloadText = $state('');
	let payload = $state<Record<string, JsonValue>>({});
	let payloadInvalid = $state(false);

	const payloadKey = (id: string, placement: WidgetPlacement) => `widgetPayload:${id}:${placement}`;

	function seedPayload(): string {
		const seed =
			draftPlacement === 'attachment'
				? Object.fromEntries(draftProps.filter(Boolean).map((p) => [p, `<${p}>`]))
				: SAMPLE;
		return JSON.stringify(seed, null, 2);
	}

	async function loadPayload() {
		const id = loadedId;
		const placement = draftPlacement;
		if (!id) return;
		payloadText = seedPayload();
		applyPayload();
		const stored = await usePlatform()
			.store.get<string>(payloadKey(id, placement))
			.catch(() => null);
		// The selection moved on while this was loading.
		if (stored === null || id !== loadedId || placement !== draftPlacement) return;
		payloadText = stored;
		applyPayload();
	}

	function editPayload() {
		applyPayload();
		if (!loadedId) return;
		const key = payloadKey(loadedId, draftPlacement);
		usePlatform()
			.store.set(key, payloadText)
			.catch((e) => showError(e, { title: t.errors.failed.persist({ key }) }));
	}

	function resetPayload() {
		payloadText = seedPayload();
		applyPayload();
		if (loadedId) void usePlatform().store.delete(payloadKey(loadedId, draftPlacement));
	}

	function applyPayload() {
		try {
			const parsed: unknown = JSON.parse(payloadText);
			payloadInvalid = parsed === null || typeof parsed !== 'object' || Array.isArray(parsed);
			if (!payloadInvalid) payload = parsed as Record<string, JsonValue>;
		} catch {
			payloadInvalid = true;
		}
	}
</script>

<div class="widgets-editor">
	<ListSidebar
		title={t.studio.widgets.title}
		detailOpen={selectedId !== null}
		oncreate={newWidget}
		{creating}
		bind:query
		bind:sort
		bind:showBuiltins
		sortOptions={[
			{ value: 'name-asc', label: t.studio.tools.sortNameAsc },
			{ value: 'name-desc', label: t.studio.tools.sortNameDesc },
			{ value: 'enabled', label: t.studio.tools.sortEnabled }
		]}
		searchPlaceholder={t.studio.widgets.search}
	>
		{#if shownWidgets.length === 0}
			<p class="empty">
				{widgets.length === 0 ? t.studio.widgets.empty : t.studio.widgets.noMatches}
			</p>
		{/if}
		{#each shownWidgets as widget (widget.id)}
			<div
				class="widget-item"
				class:selected={widget.id === selectedId}
				class:disabled={!widget.enabled}
			>
				<button class="widget-open" type="button" onclick={() => select(widget.id)}>
					<span
						class="widget-icon"
						{@attach tooltip(
							widget.placement === 'attachment'
								? t.studio.widgets.attachment
								: t.studio.widgets.widget
						)}
					>
						{#if widget.placement === 'attachment'}
							<Paperclip size={16} />
						{:else}
							<LayoutDashboard size={16} />
						{/if}
					</span>
					<span class="widget-text">
						<span class="widget-name">{widget.name || t.studio.widgets.untitled}</span>
						{#if widget.description}
							<span class="widget-sub">{widget.description}</span>
						{/if}
					</span>
				</button>
				{#if builtinIds.has(widget.id)}
					<Badge>{t.studio.tools.builtIn}</Badge>
				{:else}
					<input
						type="checkbox"
						class="switch enable-toggle"
						checked={widget.enabled}
						onchange={(e) => void userWidgetStore.setEnabled(widget.id, e.currentTarget.checked)}
						{@attach tooltip(
							widget.enabled ? t.studio.tools.enabledHint : t.studio.tools.disabledHint
						)}
						aria-label={widget.enabled
							? t.studio.tools.disableNamed({ name: widget.name })
							: t.studio.tools.enableNamed({ name: widget.name })}
					/>
				{/if}
			</div>
		{/each}
	</ListSidebar>

	<section class="col-content">
		{#if selected}
			<div class="pane">
				<header class="pane-head">
					<span
						class="kind-icon"
						{@attach tooltip(isAttachment ? t.studio.widgets.attachment : t.studio.widgets.widget)}
					>
						{#if isAttachment}
							<Paperclip size={18} />
						{:else}
							<LayoutDashboard size={18} />
						{/if}
					</span>
					<input
						class="title"
						type="text"
						aria-label={t.studio.widgets.name}
						placeholder={t.studio.widgets.untitled}
						readonly={readOnly}
						bind:value={draftName}
						oninput={requestSave}
					/>
					{#if readOnly}
						<Badge>{t.studio.tools.builtIn}</Badge>
					{:else}
						<button
							type="button"
							class="export"
							onclick={() => exportItem('widget', selected.id, selected.name)}
						>
							{t.studio.widgets.export}
						</button>
						<ConfirmDelete
							prompt={t.studio.tools.confirmDelete({
								name: selected.name || t.studio.widgets.untitled
							})}
							onConfirm={() => removeWidget(selected.id)}
						>
							{#snippet trigger(arm)}
								<button type="button" class="delete" onclick={arm}>
									{t.common.delete}
								</button>
							{/snippet}
						</ConfirmDelete>
					{/if}
				</header>

				{#if readOnly}
					<div class="readonly-note">
						<p>
							{t.studio.widgets.builtInNote}
							{#if !forkable}
								{t.studio.widgets.alreadyForkedPrefix} <code>{selected.name}</code>
								{t.studio.widgets.alreadyForkedSuffix}
							{/if}
						</p>
						{#if forkable}
							<button type="button" class="fork" onclick={forkBuiltin}>
								{t.studio.widgets.fork}
							</button>
						{/if}
					</div>
				{/if}

				<div class="workspace">
					<div class="editor">
						<div class="tabs" role="tablist" aria-label={t.studio.widgets.edit}>
							{#each TABS as key (key)}
								<button
									type="button"
									role="tab"
									aria-selected={tab === key}
									class:active={tab === key}
									onclick={() => (tab = key)}
								>
									{key === 'source'
										? t.studio.widgets.htmlSource
										: key === 'model'
											? t.studio.widgets.tabModel
											: t.studio.widgets.tabSettings}
								</button>
							{/each}
						</div>

						<div class="source" hidden={tab !== 'source'}>
							{#key syncedId}
								<HtmlEditor
									doc={draftSource}
									{readOnly}
									onChange={(v) => {
										draftSource = v;
										requestSave();
									}}
								/>
							{/key}
						</div>
						{#if tab === 'model'}
							<div class="panel">
								{#key syncedId}
									<ContextField
										label={t.studio.widgets.directive}
										value={draftPrompt}
										hint={t.studio.widgets.directiveHint}
										rows={4}
										{readOnly}
										placeholder={t.studio.widgets.directivePlaceholder}
										onchange={(v) => {
											draftPrompt = v;
											requestSave();
										}}
									/>
									<ContextField
										label={t.studio.widgets.textRendering}
										value={draftRender}
										hint={isAttachment
											? t.studio.widgets.renderHintAttachment
											: t.studio.widgets.renderHintWidget}
										rows={4}
										{readOnly}
										placeholder={isAttachment
											? t.studio.widgets.renderPlaceholderAttachment
											: t.studio.widgets.renderPlaceholderWidget}
										onchange={(v) => {
											draftRender = v;
											requestSave();
										}}
									/>
								{/key}

								<fieldset class="group" disabled={readOnly}>
									<span class="group-label">{t.studio.widgets.tools}</span>
									<span class="hint">{t.studio.widgets.toolsHint}</span>
									{#each draftTools as id (id)}
										<div class="toolRow">
											<span class="toolName">{toolName(id)}</span>
											<button
												type="button"
												class="del"
												{@attach tooltip(t.studio.widgets.removeTool)}
												onclick={() => unbindTool(id)}
											>
												×
											</button>
										</div>
									{/each}
									{#if availableTools.some((t) => !draftTools.includes(t.id))}
										<div class="bindRow">
											<select bind:value={toolPick}>
												<option value="">{t.studio.widgets.addTool}</option>
												{#each availableTools.filter((tool) => !draftTools.includes(tool.id)) as tool (tool.id)}
													<option value={tool.id}>{tool.name}</option>
												{/each}
											</select>
											<button type="button" class="bind" onclick={bindTool}>
												{t.studio.widgets.bind}
											</button>
										</div>
									{/if}
								</fieldset>
							</div>
						{:else if tab === 'settings'}
							<fieldset class="panel" disabled={readOnly}>
								<label>
									<span>{t.studio.widgets.description}</span>
									<input type="text" bind:value={draftDescription} oninput={requestSave} />
								</label>
								<label>
									<span>{t.studio.widgets.placement}</span>
									<select
										bind:value={draftPlacement}
										onchange={() => {
											void loadPayload();
											requestSave();
										}}
									>
										<option value="widget">{t.studio.widgets.placementWidget}</option>
										<option value="attachment">{t.studio.widgets.placementAttachment}</option>
									</select>
								</label>
								{#if isAttachment}
									<div class="group">
										<TagField
											items={draftProps}
											label={t.studio.widgets.properties}
											placeholder={t.studio.widgets.addProp}
											onchange={requestSave}
										/>
										<span class="hint">{t.studio.widgets.propertiesHint}</span>
									</div>
								{:else}
									<label>
										<span>{t.studio.widgets.rendersOn}</span>
										<select bind:value={draftMode} onchange={requestSave}>
											<option value="latest">{t.studio.widgets.latest}</option>
											<option value="every">{t.studio.widgets.every}</option>
										</select>
									</label>
								{/if}
							</fieldset>
						{/if}
					</div>

					<div class="preview">
						<div class="preview-head">{t.studio.widgets.preview}</div>
						<div class="preview-frame">
							<StateFrame
								source={draftSource}
								slice={payload}
								title={t.studio.widgets.previewTitle}
								kind={isAttachment ? 'props' : 'state'}
							/>
						</div>
						<details class="payload" bind:open={payloadOpen}>
							<summary>
								{t.studio.widgets.testPayload}
								{#if payloadInvalid}
									<span class="payload-error">{t.studio.widgets.payloadInvalid}</span>
								{/if}
							</summary>
							<textarea
								spellcheck="false"
								aria-label={t.studio.widgets.testPayload}
								class:invalid={payloadInvalid}
								bind:value={payloadText}
								oninput={editPayload}
							></textarea>
							<button type="button" class="payload-reset" onclick={resetPayload}>
								{t.studio.widgets.resetPayload}
							</button>
						</details>
					</div>
				</div>
			</div>
		{:else}
			<div class="pane empty-pane">
				<p class="empty">{t.studio.widgets.select}</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.widgets-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
		overflow-y: auto;
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		display: flex;
		padding: var(--spacing-lg);
	}

	.widget-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-sm);
	}
	.widget-item:hover {
		background-color: var(--color-hover);
	}
	.widget-item.selected {
		background-color: var(--color-selected);
	}
	.widget-item.disabled .widget-name {
		color: var(--color-text-muted);
	}
	.widget-open {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		background: transparent;
		padding: var(--spacing-sm);
		text-align: left;
	}
	.widget-icon {
		flex: 0 0 auto;
		display: flex;
		color: var(--color-text-muted);
	}
	.widget-item.selected .widget-icon {
		color: var(--color-text-accent);
	}
	.widget-text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.widget-name {
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.widget-sub {
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
		flex: 1;
		min-width: 0;
		min-height: 0;
		gap: var(--spacing-md);
	}
	.empty-pane {
		align-items: center;
		justify-content: center;
	}
	.pane-head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.kind-icon {
		flex: 0 0 auto;
		display: flex;
		color: var(--color-text-muted);
	}
	.title {
		flex: 1 1 auto;
		min-width: 0;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border-color: transparent;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h4);
		font-weight: var(--font-weight-semibold);
	}
	.title:hover:not([readonly]),
	.title:focus {
		background: var(--color-bg-input);
	}
	.export,
	.delete {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.delete {
		color: var(--color-error-text);
	}

	.readonly-note {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.readonly-note p {
		flex: 1;
		margin: 0;
	}
	.fork {
		flex: 0 0 auto;
		width: auto;
	}

	.workspace {
		display: flex;
		flex: 1;
		min-height: 0;
		gap: var(--spacing-lg);
	}
	.editor {
		display: flex;
		flex-direction: column;
		flex: 3 1 0;
		min-width: 0;
	}
	.preview {
		display: flex;
		flex-direction: column;
		flex: 2 1 0;
		min-width: 280px;
	}

	.tabs,
	.preview-head {
		display: flex;
		flex: 0 0 auto;
		align-items: flex-end;
		height: 36px;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tabs {
		gap: var(--spacing-xs);
	}
	.tabs button {
		padding: var(--spacing-sm) var(--spacing-md);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		border-radius: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.tabs button:hover {
		color: var(--color-text-secondary);
	}
	.tabs button.active {
		border-bottom-color: var(--color-primary-light);
		color: var(--color-text-primary);
	}
	.preview-head {
		padding: var(--spacing-sm) 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}

	.source {
		display: flex;
		flex: 1;
		min-height: 0;
		margin-top: var(--spacing-sm);
	}
	.source[hidden] {
		display: none;
	}

	.panel {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		gap: var(--spacing-lg);
		margin: 0;
		padding: var(--spacing-md) var(--spacing-xs) var(--spacing-md) 0;
		border: none;
		overflow-y: auto;
	}
	fieldset.panel {
		max-width: 560px;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		margin: 0;
		padding: 0;
		border: none;
	}
	.group-label,
	label > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.toolRow {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.toolName {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
	}
	.bindRow {
		display: flex;
		gap: var(--spacing-sm);
	}
	.bindRow > select {
		flex: 1;
		min-width: 0;
	}
	.del {
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: 0 4px;
	}
	.del:hover {
		color: var(--color-text-primary);
	}

	.preview-frame {
		flex: 1;
		min-height: 0;
		margin-top: var(--spacing-sm);
		padding: var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background: var(--color-bg-base);
		overflow-y: auto;
	}
	.payload {
		flex: 0 0 auto;
		display: flex;
		flex-direction: column;
		margin-top: var(--spacing-sm);
	}
	.payload summary {
		padding: var(--spacing-xs) 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		cursor: pointer;
	}
	.payload-error {
		margin-left: var(--spacing-sm);
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
	}
	.payload textarea {
		width: 100%;
		height: 200px;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		resize: vertical;
	}
	.payload-reset {
		align-self: flex-end;
		margin-top: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-xs);
	}
	.payload textarea.invalid {
		border-color: var(--color-error-text);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
