<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { WidgetMode, WidgetPlacement } from '$lib/types/widgets.js';
	import { onDestroy, onMount } from 'svelte';
	import ContextField from '$lib/components/ContextField.svelte';
	import StudioSidebar from '../StudioSidebar.svelte';
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

	const widgets = $derived(userWidgetStore.loadedWidgets ?? []);
	const selectedId = $derived(studioStore.editingId);
	const selected = $derived(widgets.find((w) => w.id === selectedId) ?? null);

	let query = $state('');
	let sort = $state('name-asc');
	let creating = $state(false);

	const shownWidgets = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const out = widgets.filter(
			(w) =>
				needle === '' ||
				w.name.toLowerCase().includes(needle) ||
				w.description.toLowerCase().includes(needle)
		);
		if (sort === 'name-desc') return out.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'enabled') return out.sort((a, b) => Number(b.enabled) - Number(a.enabled));
		return out.sort((a, b) => a.name.localeCompare(b.name));
	});

	// The tools a widget can bundle: the user's own tools plus the builtins.
	const availableTools = $derived([
		...(toolStore.loadedTools ?? []).map((t) => ({ id: t.id, name: t.display_name ?? t.name })),
		...(toolStore.builtinTools ?? []).map((t) => ({
			id: t.id,
			name: `${t.display_name ?? t.name} (builtin)`
		}))
	]);
	const toolName = (id: Snowflake) => availableTools.find((t) => t.id === id)?.name ?? id;

	// Local draft
	let draftName = $state('');
	let draftDescription = $state('');
	let draftSource = $state('');
	let draftPrompt = $state('');
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

	onMount(() => {
		void userWidgetStore.loadWidgets();
		if (!toolStore.loadedTools) void toolStore.loadTools();
		if (!toolStore.builtinTools) void toolStore.loadBuiltins();
	});

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void saveNow(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function saveNow() {
		const id = loadedId;
		if (!id) return;
		await userWidgetStore.updateWidget(id, {
			name: draftName,
			description: draftDescription,
			source: draftSource,
			prompt: draftPrompt,
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
		studioStore.open(id);
	}

	async function newWidget() {
		if (creating) return;
		creating = true;
		clearTimeout(timer);
		try {
			await saveNow();
			const created = await userWidgetStore.createWidget({
				name: 'New widget',
				source: STARTER
			});
			if (created) studioStore.open(created.id);
		} finally {
			creating = false;
		}
	}

	async function removeWidget(id: Snowflake, name: string) {
		if (!confirm(`Delete "${name || 'Untitled'}"? This can't be undone.`)) return;
		clearTimeout(timer);
		await userWidgetStore.deleteWidget(id);
		if (selectedId === id) studioStore.close();
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
	<StudioSidebar
		title="Widgets"
		oncreate={newWidget}
		{creating}
		bind:query
		bind:sort
		sortOptions={[
			{ value: 'name-asc', label: 'Name (A–Z)' },
			{ value: 'name-desc', label: 'Name (Z–A)' },
			{ value: 'enabled', label: 'Enabled first' }
		]}
		searchPlaceholder="Search widgets…"
	>
		{#if shownWidgets.length === 0}
			<p class="empty">
				{widgets.length === 0 ? 'No widgets yet.' : 'No widgets match that search.'}
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
						title={widget.placement === 'attachment'
							? 'Attachment'
							: 'Widget'}
					>
						{#if widget.placement === 'attachment'}
							<Paperclip size={16} />
						{:else}
							<LayoutDashboard size={16} />
						{/if}
					</span>
					<span class="widget-text">
						<span class="widget-name">{widget.name || 'Untitled'}</span>
						{#if widget.description}
							<span class="widget-sub">{widget.description}</span>
						{/if}
					</span>
				</button>
				<input
					type="checkbox"
					class="switch enable-toggle"
					checked={widget.enabled}
					onchange={(e) => void userWidgetStore.setEnabled(widget.id, e.currentTarget.checked)}
					title={widget.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'}
					aria-label={widget.enabled ? `Disable ${widget.name}` : `Enable ${widget.name}`}
				/>
			</div>
		{/each}
	</StudioSidebar>

	<section class="col-content">
		{#if selected}
			<div class="pane">
				<div class="pane-head">
					<h1>Edit widget</h1>
					<button
						type="button"
						class="delete"
						onclick={() => void removeWidget(selected.id, selected.name)}
					>
						Delete
					</button>
				</div>

				<div class="fields">
					<label>
						<span>Name</span>
						<input type="text" bind:value={draftName} oninput={requestSave} />
					</label>
					<label>
						<span>
							{isAttachment ? 'Description' : 'Description'}
						</span>
						<input type="text" bind:value={draftDescription} oninput={requestSave} />
					</label>
					<label>
						<span>Placement</span>
						<select bind:value={draftPlacement} onchange={requestSave}>
							<option value="widget">Widget — always on, reads state</option>
							<option value="attachment">Attachment — the model places it inline</option>
						</select>
					</label>
					{#if !isAttachment}
						<label>
							<span>Renders on</span>
							<select bind:value={draftMode} onchange={requestSave}>
								<option value="latest">Latest message only</option>
								<option value="every">Every message (costly in long chats)</option>
							</select>
						</label>
					{/if}
				</div>

				{#if isAttachment}
					<div class="props">
						<span class="propsLabel">
							Properties
							<code>{'{ type: "lorealm:props", props }'}</code>.
						</span>
						{#each draftProps, i (i)}
							<div class="propRow">
								<input
									type="text"
									placeholder="name"
									bind:value={draftProps[i]}
									oninput={requestSave}
								/>
								<button type="button" class="del" title="Remove prop" onclick={() => removeProp(i)}>
									×
								</button>
							</div>
						{/each}
						<button type="button" class="new" onclick={addProp}>Add prop</button>
					</div>
				{/if}

				<div class="split">
					<label class="sourceBox">
						<span>
							HTML source
						</span>
						<textarea spellcheck="false" bind:value={draftSource} oninput={requestSave}></textarea>
					</label>

					<div class="previewBox">
						<span>Preview (sample state)</span>
						{#key draftSource}
							<iframe
								title="Widget preview"
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
							label="Directive"
							value={draftPrompt}
							hint = "A weaver-formatted prompt sent to the model. Describes how the model should maintain this "
							rows={3}
							placeholder="When affection changes, update relationship:&lbrace;&lbrace;char:name&rbrace;&rbrace;.affection using the set_var tool."
							onchange={(v) => {
								draftPrompt = v;
								requestSave();
							}}
						/>
					{/key}
				</div>

				<div class="tools">
					<span class="toolsLabel">
						Tools — made available to the model whenever this widget is active. Imported into a
						realm with the widget; no separate tool import needed.
					</span>
					{#each draftTools as id (id)}
						<div class="toolRow">
							<span class="toolName">{toolName(id)}</span>
							<button type="button" class="del" title="Remove tool" onclick={() => unbindTool(id)}>
								×
							</button>
						</div>
					{/each}
					{#if availableTools.some((t) => !draftTools.includes(t.id))}
						<div class="bindRow">
							<select bind:value={toolPick}>
								<option value="">Add a tool…</option>
								{#each availableTools.filter((t) => !draftTools.includes(t.id)) as t (t.id)}
									<option value={t.id}>{t.name}</option>
								{/each}
							</select>
							<button type="button" class="bind" onclick={bindTool}>Bind</button>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="pane empty-pane">
				<p class="empty">Select a widget to edit, or create one.</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.widgets-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
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
		flex: 1;
		min-height: 0;
		gap: var(--spacing-md);
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
