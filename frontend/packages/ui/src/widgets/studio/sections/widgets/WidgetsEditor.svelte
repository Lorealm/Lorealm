<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { navigator } from '#lib/navigation/navigator.js';
	import { toolStore } from '#lib/stores/tools.svelte.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import { exportItem } from '#lib/stores/transfer.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { WidgetMode, WidgetPlacement } from '#lib/types/widgets.js';
	import { onDestroy } from 'svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import ListSidebar from '#lib/components/ListSidebar.svelte';
	import Badge from '#lib/widgets/realms/Badge.svelte';
	import { LayoutDashboard, Paperclip } from '@lucide/svelte';

	const SAVE_MS = 500;

	/** Skeleton for a new widget: reads the state slice the host posts in. */
	const STARTER = `<!doctype html>
<meta charset="utf-8" />
<style>
  body { margin: 0; font: 13px system-ui, sans-serif; color: CanvasText; background: Canvas; }
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

  // To act, post back a bound tool call — it runs as an ordinary player turn:
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
			name: t.studio.widgets.builtinSuffix(tool.display_name ?? tool.name)
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

	function addProp() {
		draftProps = [...draftProps, ''];
	}
	function removeProp(i: number) {
		draftProps = draftProps.filter((_, k) => k !== i);
		requestSave();
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

	async function removeWidget(id: Snowflake, name: string) {
		if (!confirm(t.studio.tools.confirmDelete(name || t.studio.widgets.untitled))) return;
		clearTimeout(timer);
		await userWidgetStore.deleteWidget(id);
		if (selectedId === id) navigator.start().studio().widgets();
	}

	let previewFrame = $state<HTMLIFrameElement | null>(null);

	const SAMPLE = {
		world: { season: 'winter' },
		self: { stamina: 3, inventory: { sword: 1, potion: 2 } }
	};

	const previewPayload = $derived(
		isAttachment ? Object.fromEntries(draftProps.filter(Boolean).map((p) => [p, `<${p}>`])) : SAMPLE
	);
	function feedPreview() {
		const payload = $state.snapshot(previewPayload);
		previewFrame?.contentWindow?.postMessage(
			{
				type: isAttachment ? 'lorealm:props' : 'lorealm:state',
				state: payload,
				props: payload
			},
			'*'
		);
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
							? t.studio.tools.disableNamed(widget.name)
							: t.studio.tools.enableNamed(widget.name)}
					/>
				{/if}
			</div>
		{/each}
	</ListSidebar>

	<section class="col-content">
		{#if selected}
			<div class="pane">
				<div class="pane-head">
					<h1>{readOnly ? t.studio.widgets.builtInWidget : t.studio.widgets.edit}</h1>
					{#if !readOnly}
						<button
							type="button"
							class="export"
							onclick={() => exportItem('widget', selected.id, selected.name)}
						>
							{t.studio.widgets.export}
						</button>
						<button
							type="button"
							class="delete"
							onclick={() => void removeWidget(selected.id, selected.name)}
						>
							{t.common.delete}
						</button>
					{/if}
				</div>

				{#if readOnly}
					<div class="readonly-note">
						<p>{t.studio.widgets.builtInNote}</p>
						{#if forkable}
							<button type="button" class="fork" onclick={forkBuiltin}
								>{t.studio.widgets.fork}</button
							>
						{:else}
							<p class="already-forked">
								{t.studio.widgets.alreadyForkedPrefix} <code>{selected.name}</code>
								{t.studio.widgets.alreadyForkedSuffix}
							</p>
						{/if}
					</div>
				{/if}

				<fieldset class="body" disabled={readOnly}>
					<div class="fields">
						<label>
							<span>{t.studio.widgets.name}</span>
							<input type="text" bind:value={draftName} oninput={requestSave} />
						</label>
						<label>
							<span>
								{t.studio.widgets.description}
							</span>
							<input type="text" bind:value={draftDescription} oninput={requestSave} />
						</label>
						<label>
							<span>{t.studio.widgets.placement}</span>
							<select bind:value={draftPlacement} onchange={requestSave}>
								<option value="widget">{t.studio.widgets.placementWidget}</option>
								<option value="attachment">{t.studio.widgets.placementAttachment}</option>
							</select>
						</label>
						{#if !isAttachment}
							<label>
								<span>{t.studio.widgets.rendersOn}</span>
								<select bind:value={draftMode} onchange={requestSave}>
									<option value="latest">{t.studio.widgets.latest}</option>
									<option value="every">{t.studio.widgets.every}</option>
								</select>
							</label>
						{/if}
					</div>

					{#if isAttachment}
						<div class="props">
							<span class="propsLabel">
								{t.studio.widgets.properties}
								<code>{'{ type: "lorealm:props", props }'}</code>.
							</span>
							{#each draftProps, i (i)}
								<div class="propRow">
									<input
										type="text"
										placeholder={t.studio.widgets.propPlaceholder}
										bind:value={draftProps[i]}
										oninput={requestSave}
									/>
									<button
										type="button"
										class="del"
										{@attach tooltip(t.studio.widgets.removeProp)}
										onclick={() => removeProp(i)}
									>
										×
									</button>
								</div>
							{/each}
							<button type="button" class="new" onclick={addProp}>{t.studio.widgets.addProp}</button
							>
						</div>
					{/if}

					<div class="split">
						<label class="sourceBox">
							<span>{t.studio.widgets.htmlSource}</span>
							<textarea spellcheck="false" bind:value={draftSource} oninput={requestSave}
							></textarea>
						</label>

						<div class="previewBox">
							<span>{t.studio.widgets.preview}</span>
							{#key draftSource}
								<iframe
									title={t.studio.widgets.previewTitle}
									sandbox="allow-scripts"
									srcdoc={draftSource}
									bind:this={previewFrame}
									onload={feedPreview}
								></iframe>
							{/key}
						</div>
					</div>

					<div class="prompt">
						{#key syncedId}
							<ContextField
								label={t.studio.widgets.directive}
								value={draftPrompt}
								hint={t.studio.widgets.directiveHint}
								rows={3}
								{readOnly}
								placeholder={t.studio.widgets.directivePlaceholder}
								onchange={(v) => {
									draftPrompt = v;
									requestSave();
								}}
							/>
						{/key}
					</div>

					<div class="prompt">
						{#key syncedId}
							<ContextField
								label={t.studio.widgets.textRendering}
								value={draftRender}
								hint={isAttachment
									? t.studio.widgets.renderHintAttachment
									: t.studio.widgets.renderHintWidget}
								rows={3}
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
					</div>

					<div class="tools">
						<span class="toolsLabel">
							{t.studio.widgets.tools}
						</span>
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
								<button type="button" class="bind" onclick={bindTool}
									>{t.studio.widgets.bind}</button
								>
							</div>
						{/if}
					</div>
				</fieldset>
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

	.del {
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: 0 4px;
	}

	.del:hover {
		color: var(--color-text-primary);
	}

	.pane {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
		gap: var(--spacing-md);
		overflow-y: auto;
	}
	.pane-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.pane h1 {
		font-size: var(--font-size-h3);
		margin: 0;
	}
	.empty-pane {
		align-items: center;
		justify-content: center;
	}
	.body {
		display: contents;
	}
	.readonly-note {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.readonly-note p {
		margin: 0;
	}
	.fork {
		width: auto;
	}
	.already-forked {
		color: var(--color-text-muted);
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

	.fields {
		display: flex;
		gap: var(--spacing-md);
	}

	.fields > label {
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

	.split {
		display: flex;
		flex: 1 0 auto;
		min-height: 300px;
		gap: var(--spacing-md);
	}

	.prompt,
	.tools,
	.props {
		flex-shrink: 0;
	}

	.tools,
	.props {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.toolsLabel,
	.propsLabel {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.propRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
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

	.sourceBox {
		flex: 1;
		min-width: 0;
	}

	.sourceBox textarea {
		flex: 1;
		min-height: 0;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		resize: none;
	}

	.previewBox {
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 320px;
		flex-shrink: 0;
	}

	.previewBox > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.previewBox > iframe {
		flex: 1;
		min-height: 0;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background: var(--color-bg-elevated);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
