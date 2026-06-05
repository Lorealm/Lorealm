<script lang="ts">
	import type { CreateNodeRequest } from '$lib/types/template.js';
	import type {
		GroupMode,
		SourceDefinition,
		SourceNodeDefinition
	} from '$lib/types/node.js';
	import SourcePicker from './SourcePicker.svelte';
	import type { JsonValue, PromptRole } from '$lib/types/core.js';

	type Kind = 'text' | 'group' | 'source';

	let {
		kind,
		sources,
		existing,
		onclose,
		oncreate
	}: {
		kind: Kind;
		sources: SourceDefinition[] | undefined;
		existing: Set<string>;
		onclose: () => void;
		/** Persists the node. Resolves on success (parent then closes), rejects on failure. */
		oncreate: (req: CreateNodeRequest) => Promise<void>;
	} = $props();

	let submitting = $state(false);
	let error = $state<string | null>(null);

	// shared
	let title = $state('');
	let description = $state('');

	// text
	let role = $state<PromptRole>('system');
	let textContent = $state('');

	// group
	let mode = $state<GroupMode>('collection');
	let prefix = $state('');
	let suffix = $state('');
	let alwaysIncludeWrappers = $state(false);

	// source
	let picked = $state<{ source_id: string; node_id: string; def: SourceNodeDefinition } | null>(
		null
	);

	const heading = $derived(
		kind === 'text' ? 'New text node' : kind === 'group' ? 'New group' : 'Add source node'
	);
	const showForm = $derived(!(kind === 'source' && !picked));
	const canSubmit = $derived(!submitting && showForm && title.trim().length > 0);

	function onPicked(sel: { source_id: string; node_id: string; def: SourceNodeDefinition }) {
		picked = sel;
		title = sel.def.title;
		description = sel.def.description;
	}

	function buildRequest(): CreateNodeRequest | null {
		const base = { title: title.trim(), description: description.trim() };
		if (kind === 'text') {
			return { ...base, content: { type: 'text', content: textContent, role } };
		}
		if (kind === 'group') {
			return {
				...base,
				content: {
					type: 'group',
					children: [],
					prefix: prefix.trim() ? prefix : null,
					suffix: suffix.trim() ? suffix : null,
					always_include_wrappers: alwaysIncludeWrappers,
					mode
				}
			};
		}
		if (!picked) return null;
		const settings: Record<string, JsonValue> = {};
		for (const s of picked.def.settings) settings[s.key] = s.default_value;
		return {
			...base,
			content: { type: 'source', source_id: picked.source_id, node_id: picked.node_id, settings }
		};
	}

	async function submit() {
		if (!canSubmit) return;
		const req = buildRequest();
		if (!req) return;
		submitting = true;
		error = null;
		try {
			await oncreate(req);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to add node.';
			submitting = false;
		}
	}

	function focusOnMount(node: HTMLElement) {
		node.focus();
	}

	$effect(() => {
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') onclose();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="overlay"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="panel" role="dialog" aria-modal="true" aria-label={heading}>
		<header class="panel-header">
			<h3>{heading}</h3>
			<button class="close" type="button" aria-label="Close" onclick={onclose}>×</button>
		</header>

		<div class="panel-body">
			{#if kind === 'source' && !picked}
				<SourcePicker {sources} {existing} onpick={onPicked} />
			{:else}
				{#if kind === 'source' && picked}
					<button class="link" type="button" onclick={() => (picked = null)}>
						← Choose a different source
					</button>
				{/if}

				<label class="field">
					<span>Title</span>
					<input use:focusOnMount bind:value={title} placeholder="Title" />
				</label>

				<label class="field">
					<span>Description</span>
					<textarea bind:value={description} rows="2" placeholder="Optional"></textarea>
				</label>

				{#if kind === 'text'}
					<label class="field">
						<span>Role</span>
						<select bind:value={role}>
							<option value="system">System</option>
							<option value="user">User</option>
							<option value="assistant">Assistant</option>
						</select>
					</label>
					<label class="field">
						<span>Content</span>
						<textarea
							bind:value={textContent}
							rows="4"
							placeholder="Optional — you can edit this later"
						></textarea>
					</label>
				{/if}

				{#if kind === 'group'}
					<label class="field">
						<span>Mode</span>
						<select bind:value={mode}>
							<option value="collection">Collection</option>
							<option value="invariant">Invariant</option>
						</select>
					</label>
					<label class="field">
						<span>Prefix</span>
						<textarea
							bind:value={prefix}
							rows="2"
							placeholder="Optional (prepended before children)"
						></textarea>
					</label>
					<label class="field">
						<span>Suffix</span>
						<textarea bind:value={suffix} rows="2" placeholder="Optional (appended after children)"
						></textarea>
					</label>
					<label class="check">
						<input type="checkbox" bind:checked={alwaysIncludeWrappers} />
						<span>Always include prefix/suffix, even if all children are disabled</span>
					</label>
				{/if}

				{#if kind === 'source' && picked}
					<p class="hint">
						{picked.def.settings.length} setting{picked.def.settings.length === 1 ? '' : 's'} set to defaults.
					</p>
				{/if}
			{/if}

			{#if error}
				<p class="error-text">{error}</p>
			{/if}
		</div>

		{#if showForm}
			<footer class="panel-footer">
				<button class="ghost" type="button" onclick={onclose} disabled={submitting}>Cancel</button>
				<button class="primary" type="button" onclick={submit} disabled={!canSubmit}>
					{submitting ? 'Adding…' : 'Add'}
				</button>
			</footer>
		{/if}
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-lg);
		background-color: var(--color-overlay-bg);
		backdrop-filter: var(--backdrop-blur-sm);
	}

	.panel {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: var(--width-content-narrow);
		max-height: 80vh;
		background-color: var(--color-bg-modal);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-xl);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.panel-header h3 {
		margin: 0;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h4);
	}
	.close {
		flex: 0 0 auto;
		background-color: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-lg);
		line-height: 1;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.close:hover {
		color: var(--color-text-accent);
	}

	.panel-body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		overflow-y: auto;
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

	.check {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	/* override the global full-width input styling for the checkbox */
	.check input[type='checkbox'] {
		width: auto;
		flex: 0 0 auto;
		margin-top: 2px;
	}

	.hint {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.error-text {
		margin: 0;
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}

	.link {
		align-self: flex-start;
		background-color: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.link:hover {
		color: var(--color-text-accent);
	}

	.panel-footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
	}
	.primary {
		background-color: var(--color-primary-dark);
		color: var(--color-text-white);
	}
	.primary:hover:not(:disabled) {
		background-color: var(--color-primary);
	}
	.primary:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.ghost {
		background-color: transparent;
		color: var(--color-text-secondary);
	}
	.ghost:hover:not(:disabled) {
		background-color: var(--color-hover);
	}
</style>
