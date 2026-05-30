<script lang="ts">
	import { untrack } from 'svelte';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import type {
		ContextNode,
		GroupMode,
		JsonValue,
		NodeContent,
		PromptRole,
		SourceNodeDefinition
	} from '$lib/types/node.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { UpdateNodeRequest } from '$lib/types/template.js';
	import { SvelteSet } from 'svelte/reactivity';

	type SaveState = 'idle' | 'saving' | 'saved' | 'error';

	let {
		node,
		saveState = $bindable<SaveState>('idle')
	}: {
		node: ContextNode;
		saveState?: SaveState;
	} = $props();

	const DEBOUNCE = 500;

	// Local mirrors. Synced from `node` only when the selected node changes, so a
	// server reconcile mid-edit can't clobber what the user is typing.
	let title = $state('');
	let description = $state('');
	let role = $state<PromptRole>('system');
	let mode = $state<GroupMode>('collection');
	let prefix = $state('');
	let suffix = $state('');
	let alwaysIncludeWrappers = $state(false);
	let settings = $state<Record<string, JsonValue>>({});

	let lastId: Snowflake | null = null;
	let timer: ReturnType<typeof setTimeout> | null = null;
	const dirty = new SvelteSet<'title' | 'description' | 'content'>();

	$effect(() => {
		const n = node; // track node identity only
		const nid = n.id;
		if (nid === lastId) return;
		untrack(() => {
			void flush(); // persist edits from the node we're leaving
			lastId = nid;
			saveState = 'idle';
			title = n.title;
			description = n.description;
			const c = n.content;
			if (c.type === 'text') role = c.role;
			if (c.type === 'group') {
				mode = c.mode;
				prefix = c.prefix ?? '';
				suffix = c.suffix ?? '';
				alwaysIncludeWrappers = c.always_include_wrappers;
			}
			if (c.type === 'source') settings = { ...c.settings };
		});
	});

	const sourceDef = $derived.by((): SourceNodeDefinition | null => {
		if (node.content.type !== 'source') return null;
		const { source_id, node_id } = node.content;
		const src = templateStore.sources?.find((s) => s.source_id === source_id);
		return src?.nodes.find((sn) => sn.node_id === node_id) ?? null;
	});

	// Rebuild full NodeContent, preserving the fields this form doesn't edit
	// (text body, group children, source identity).
	function buildContent(): NodeContent {
		const c = node.content;
		if (c.type === 'text') return { type: 'text', content: c.content, role };
		if (c.type === 'group') {
			return {
				type: 'group',
				children: c.children,
				prefix: prefix.trim() ? prefix : null,
				suffix: suffix.trim() ? suffix : null,
				always_include_wrappers: alwaysIncludeWrappers,
				mode
			};
		}
		return {
			type: 'source',
			source_id: c.source_id,
			node_id: c.node_id,
			settings: { ...settings }
		};
	}

	function scheduleSave(field: 'title' | 'description' | 'content', immediate = false) {
		dirty.add(field);
		saveState = 'saving';
		if (timer) clearTimeout(timer);
		if (immediate) void flush();
		else timer = setTimeout(() => void flush(), DEBOUNCE);
	}

	async function flush() {
		if (timer) {
			clearTimeout(timer);
			timer = null;
		}
		if (dirty.size === 0) return;
		const update: UpdateNodeRequest = {};
		if (dirty.has('title')) update.title = title.trim();
		if (dirty.has('description')) update.description = description.trim();
		if (dirty.has('content')) update.content = buildContent();
		dirty.clear();
		try {
			await templateStore.updateNode(node.id, update);
			saveState = 'saved';
		} catch {
			saveState = 'error';
		}
	}

	function onNumberSetting(key: string, raw: string, integer: boolean) {
		if (raw.trim() === '') return;
		const num = integer ? parseInt(raw, 10) : parseFloat(raw);
		if (Number.isNaN(num)) return;
		settings[key] = num;
		scheduleSave('content');
	}
</script>

<div class="settings">
	{#if node.content.type === 'source'}
		<p class="source-line">From source · {sourceDef ? sourceDef.title : 'Unknown source node'}</p>
	{/if}

	<label class="field">
		<span>Title</span>
		<input
			value={title}
			placeholder="Title"
			oninput={(e) => {
				title = e.currentTarget.value;
				scheduleSave('title');
			}}
		/>
	</label>

	<label class="field">
		<span>Description</span>
		<textarea
			value={description}
			rows="2"
			placeholder="Optional"
			oninput={(e) => {
				description = e.currentTarget.value;
				scheduleSave('description');
			}}
		></textarea>
	</label>

	{#if node.content.type === 'text'}
		<label class="field">
			<span>Role</span>
			<select
				value={role}
				onchange={(e) => {
					role = e.currentTarget.value as PromptRole;
					scheduleSave('content', true);
				}}
			>
				<option value="system">System</option>
				<option value="user">User</option>
				<option value="assistant">Assistant</option>
			</select>
		</label>
	{/if}

	{#if node.content.type === 'group'}
		<label class="field">
			<span>Mode</span>
			<select
				value={mode}
				onchange={(e) => {
					mode = e.currentTarget.value as GroupMode;
					scheduleSave('content', true);
				}}
			>
				<option value="collection">Collection</option>
				<option value="invariant">Invariant (one active child)</option>
			</select>
		</label>
		<label class="field">
			<span>Prefix</span>
			<textarea
				value={prefix}
				rows="2"
				placeholder="Optional — prepended before children"
				oninput={(e) => {
					prefix = e.currentTarget.value;
					scheduleSave('content');
				}}
			></textarea>
		</label>
		<label class="field">
			<span>Suffix</span>
			<textarea
				value={suffix}
				rows="2"
				placeholder="Optional — appended after children"
				oninput={(e) => {
					suffix = e.currentTarget.value;
					scheduleSave('content');
				}}
			></textarea>
		</label>
		<label class="check">
			<input
				type="checkbox"
				checked={alwaysIncludeWrappers}
				onchange={(e) => {
					alwaysIncludeWrappers = e.currentTarget.checked;
					scheduleSave('content', true);
				}}
			/>
			<span>Always include prefix/suffix, even if all children are disabled</span>
		</label>
	{/if}

	{#if node.content.type === 'source'}
		<div class="custom">
			<div class="section-title">Source settings</div>
			{#if !sourceDef}
				<p class="muted">Source definition unavailable — settings can't be shown.</p>
			{:else if sourceDef.settings.length === 0}
				<p class="muted">This source node has no settings.</p>
			{:else}
				{#each sourceDef.settings as s (s.key)}
					{@const st = s.setting_type}
					<div class="field">
						<span
							>{s.label}{#if s.required}<em class="req"> *</em>{/if}</span
						>

						{#if st.type === 'boolean'}
							<input
								type="checkbox"
								checked={settings[s.key] === true}
								onchange={(e) => {
									settings[s.key] = e.currentTarget.checked;
									scheduleSave('content', true);
								}}
							/>
						{:else if st.type === 'select'}
							<select
								value={String(settings[s.key] ?? '')}
								onchange={(e) => {
									settings[s.key] = e.currentTarget.value;
									scheduleSave('content', true);
								}}
							>
								{#each st.options as opt (opt.value)}
									<option value={opt.value}>{opt.label}</option>
								{/each}
							</select>
						{:else if st.type === 'integer'}
							<input
								type="number"
								step="1"
								min={st.min ?? undefined}
								max={st.max ?? undefined}
								value={(settings[s.key] as number) ?? ''}
								oninput={(e) => onNumberSetting(s.key, e.currentTarget.value, true)}
							/>
						{:else if st.type === 'token_count'}
							<input
								type="number"
								step="1"
								min="0"
								value={(settings[s.key] as number) ?? ''}
								oninput={(e) => onNumberSetting(s.key, e.currentTarget.value, true)}
							/>
						{:else if st.type === 'float'}
							<input
								type="number"
								step="any"
								min={st.min ?? undefined}
								max={st.max ?? undefined}
								value={(settings[s.key] as number) ?? ''}
								oninput={(e) => onNumberSetting(s.key, e.currentTarget.value, false)}
							/>
						{:else}
							<input
								value={String(settings[s.key] ?? '')}
								oninput={(e) => {
									settings[s.key] = e.currentTarget.value;
									scheduleSave('content');
								}}
							/>
						{/if}

						{#if s.description}<small class="desc">{s.description}</small>{/if}
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>

<style>
	.settings {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}
	.source-line {
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.field > span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.req {
		color: var(--color-warning-text);
		font-style: normal;
	}
	.desc {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	/* the global input rule forces full width/elevated bg; checkboxes opt out */
	input[type='checkbox'] {
		width: auto;
		flex: 0 0 auto;
		margin-top: 2px;
	}

	.custom {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding-top: var(--spacing-md);
		border-top: 1px solid var(--color-border-subtle);
	}
	.section-title {
		color: var(--color-text-accent);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-sm);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-wide);
	}
	.muted {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
