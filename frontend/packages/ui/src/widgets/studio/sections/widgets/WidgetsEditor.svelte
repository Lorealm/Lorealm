<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { WidgetMode, WidgetPlacement } from '$lib/types/widgets.js';
	import { onDestroy, onMount } from 'svelte';
	import ContextField from '$lib/components/ContextField.svelte';

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
		clearTimeout(timer);
		await saveNow();
		const created = await userWidgetStore.createWidget({
			name: 'New widget',
			source: STARTER
		});
		if (created) studioStore.open(created.id);
	}

	async function removeWidget(id: Snowflake, name: string) {
		if (!confirm(`Delete "${name || 'Untitled'}"? This can't be undone.`)) return;
		clearTimeout(timer);
		await userWidgetStore.deleteWidget(id);
		if (selectedId === id) studioStore.close();
	}

	// Preview: re-mount the iframe when the source changes, then feed it a
	// sample payload so the author sees the shape their widget receives.
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

<div class="editor">
	<section class="list">
		<header>
			<span class="title">Widgets</span>
			<button type="button" class="new" onclick={newWidget}>New</button>
		</header>
		<nav>
			{#each widgets as widget (widget.id)}
				<div class="row" class:active={widget.id === selectedId}>
					<button type="button" class="pick" onclick={() => select(widget.id)}>
						<span class="name">{widget.name || 'Untitled'}</span>
					</button>
					<label class="toggle" title="Show in every conversation">
						<input
							type="checkbox"
							checked={widget.enabled}
							onchange={(e) => void userWidgetStore.setEnabled(widget.id, e.currentTarget.checked)}
						/>
					</label>
					<button
						type="button"
						class="del"
						title="Delete widget"
						onclick={() => void removeWidget(widget.id, widget.name)}
					>
						×
					</button>
				</div>
			{:else}
				<div class="empty">No widgets yet.</div>
			{/each}
		</nav>
	</section>

	<section class="pane">
		{#if selected}
			<div class="fields">
				<label>
					<span>Name</span>
					<input type="text" bind:value={draftName} oninput={requestSave} />
				</label>
				<label>
					<span>
						{isAttachment ? 'Description — what the model searches against' : 'Description'}
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
						Props — the values the model fills in. Your HTML receives them as
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
						HTML source — receives
						<code
							>{isAttachment
								? '{ type: "lorealm:props", props }'
								: '{ type: "lorealm:state", state }'}</code
						>
						via postMessage
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
				<span>
					Model directive — how the model should maintain this widget's state. Requires the
					<code>Widget Directives</code> node in the active context template to reach the model.
				</span>
				{#key syncedId}
					<ContextField
						label="Directive"
						value={draftPrompt}
						rows={3}
						placeholder="e.g. When affection changes, update relationship:<name>.affection (0–100)."
						onchange={(v) => {
							draftPrompt = v;
							requestSave();
						}}
					/>
				{/key}
			</div>

			<div class="tools">
				<span class="toolsLabel">
					Tools — made available to the model whenever this widget is active. Imported into a realm
					with the widget; no separate tool import needed.
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
		{:else}
			<div class="empty pad">Select a widget, or create one.</div>
		{/if}
	</section>
</div>

<style>
	.editor {
		display: flex;
		flex: 1;
		min-height: 0;
		gap: var(--spacing-md);
	}

	.list {
		display: flex;
		flex-direction: column;
		width: 240px;
		flex-shrink: 0;
		gap: var(--spacing-sm);
	}

	.list > header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.title {
		font-family: var(--font-family-heading);
		font-weight: 600;
	}

	.list > nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow-y: auto;
	}

	.row {
		display: flex;
		align-items: center;
		gap: 4px;
		border-radius: var(--radius-md);
		padding: 2px 4px;
	}

	.row.active {
		background: var(--color-bg-elevated);
	}

	.pick {
		flex: 1;
		min-width: 0;
		text-align: left;
		background: none;
		border: none;
		color: inherit;
		padding: 4px;
	}

	.name {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.toggle input {
		width: auto;
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

	.prompt code {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
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

	.pad {
		padding: var(--spacing-md);
	}
</style>
