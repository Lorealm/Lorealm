<script lang="ts">
	import { lorebookStore, type DraftEntry, type SaveState } from '$lib/stores/lorebook.svelte.js';
	import type { EntryMeta, LorebookConfig, Slot } from '$lib/types/lorebook.js';
	import type { ReferenceSource } from '$lib/weaver/editor/weaver-references.js';
	import WeaverInput from './WeaverInput.svelte';
	import StringListField from './StringListField.svelte';
	import SlotPicker from '../SlotPicker.svelte';
	import SaveIndicator from '$lib/components/SaveIndicator.svelte';
	import NumberInput from '$lib/components/NumberInput.svelte';
	import WeaverEditor from '$lib/weaver/editor/WeaverEditor.svelte';

	let {
		entry,
		config,
		saveState,
		requestSave
	}: {
		entry: DraftEntry;
		config: LorebookConfig | null;
		saveState: SaveState;
		requestSave: () => void;
	} = $props();

	// Read through the store rather than off a snapshot, so ids stay current as
	// entries are added or renamed while this editor is open.
	const references: ReferenceSource = {
		entries: () => lorebookStore.referenceableEntries,
		navigate: (ref) => lorebookStore.navigateToEntry(ref.id)
	};

	function set<K extends keyof EntryMeta>(key: K, value: EntryMeta[K]) {
		entry.meta[key] = value;
		requestSave();
	}

	function setOptStr<K extends keyof EntryMeta>(key: K, v: string) {
		set(key, (v === '' ? undefined : v) as EntryMeta[K]);
	}
	function setBody(v: string) {
		entry.body = v;
		requestSave();
	}

	const fallback = $derived(entry.meta.fallback ?? []);
	function setFallback(i: number, v: Slot | undefined) {
		const next = [...fallback];
		if (v) next[i] = v;
		set('fallback', next);
	}
	function addFallback() {
		set('fallback', [...fallback, 'backdrop']);
	}
	function removeFallback(i: number) {
		const next = fallback.filter((_, j) => j !== i);
		set('fallback', next.length > 0 ? next : undefined);
	}
</script>

<div class="entry-editor">
	<header class="head">
		<div class="head-fields">
			<label class="field id">
				<span>ID</span>
				<input
					type="text"
					value={entry.meta.id}
					oninput={(e) => set('id', e.currentTarget.value)}
				/>
			</label>
			<label class="field name">
				<span>Name</span>
				<input
					type="text"
					value={entry.meta.name ?? ''}
					oninput={(e) => setOptStr('name', e.currentTarget.value)}
				/>
			</label>
			<label class="field enabled">
				<input
					type="checkbox"
					checked={entry.meta.enabled ?? true}
					onchange={(e) => set('enabled', e.currentTarget.checked)}
				/>
				<span>Enabled</span>
			</label>
		</div>
		<SaveIndicator state={saveState} />
	</header>

	<section class="block">
		<div class="block-title">Content</div>
		<WeaverEditor
			doc={entry.body}
			onChange={setBody}
			{references}
		/>
	</section>

	<fieldset class="block activation">
		<legend>Activation</legend>

		<div class="field">
			<span>Keywords</span>
			<StringListField
				values={entry.meta.keywords}
				onchange={(v) => set('keywords', v)}
				placeholder="keyword"
				ariaLabel="Keywords"
			/>
		</div>

		<div class="field">
			<span>Regex</span>
			<StringListField
				values={entry.meta.regex}
				onchange={(v) => set('regex', v)}
				placeholder="pattern"
				ariaLabel="Regex patterns"
			/>
		</div>

		<div class="field">
			<span>Condition</span>
			<WeaverInput
				value={entry.meta.condition ?? ''}
				onchange={(v) => setOptStr('condition', v)}
				placeholder="weaver expression, e.g. {'{{state:level}}'} > 5"
				ariaLabel="Activation condition"
				{references}
			/>
		</div>

		<div class="field-row">
			<label class="field inline">
				<input
					type="checkbox"
					checked={entry.meta.constant ?? false}
					onchange={(e) => set('constant', e.currentTarget.checked)}
				/>
				<span>Constant (always active)</span>
			</label>
			<label class="field scan">
				<span>Scan depth</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.scan_depth ?? ''}
					placeholder={config?.default_scan_depth?.toString() ?? ''}
					onchange={(v) => set('scan_depth', v === '' ? undefined : v)}
					ariaLabel="Scan depth"
				/>
			</label>
		</div>
	</fieldset>

	<details class="block advanced">
		<summary>Advanced</summary>

		<div class="field-row">
			<label class="field">
				<span>Priority</span>
				<NumberInput
					step={1}
					value={entry.meta.priority ?? ''}
					placeholder={config?.default_priority?.toString() ?? ''}
					onchange={(v) => set('priority', v === '' ? undefined : v)}
					ariaLabel="Priority"
				/>
			</label>
			<label class="field">
				<span>Insertion order</span>
				<NumberInput
					step={1}
					value={entry.meta.insertion_order ?? ''}
					onchange={(v) => set('insertion_order', v === '' ? undefined : v)}
					ariaLabel="Insertion order"
				/>
			</label>
		</div>

		<div class="field">
			<span>Slot</span>
			<SlotPicker
				value={entry.meta.slot}
				inheritedDefault={config?.default_slot}
				onchange={(v) => set('slot', v)}
				ariaLabel="Slot"
			/>
		</div>

		<div class="field">
			<span>Fallback slots</span>
			<div class="fallback-list">
				{#each fallback as fb, i (i)}
					<div class="fallback-row">
						<SlotPicker
							value={fb}
							allowInherit={false}
							onchange={(v) => setFallback(i, v)}
							ariaLabel={`Fallback ${i + 1}`}
						/>
						<button
							type="button"
							class="remove"
							aria-label="Remove"
							onclick={() => removeFallback(i)}>×</button
						>
					</div>
				{/each}
				<button type="button" class="add" onclick={addFallback}>+ Add fallback</button>
			</div>
		</div>

		<div class="field-row">
			<label class="field">
				<span>Sticky turns</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.sticky_turns ?? ''}
					onchange={(v) => set('sticky_turns', v === '' ? undefined : v)}
					ariaLabel="Sticky turns"
				/>
			</label>
			<label class="field">
				<span>Cooldown</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.cooldown ?? ''}
					onchange={(v) => set('cooldown', v === '' ? undefined : v)}
					ariaLabel="Cooldown"
				/>
			</label>
		</div>

		<label class="field">
			<span>Group</span>
			<input
				type="text"
				value={entry.meta.group ?? ''}
				oninput={(e) => setOptStr('group', e.currentTarget.value)}
			/>
		</label>

		<div class="field">
			<span>Tags</span>
			<StringListField
				values={entry.meta.tags}
				onchange={(v) => set('tags', v)}
				placeholder="tag"
				ariaLabel="Tags"
			/>
		</div>
	</details>
</div>

<style>
	.entry-editor {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		min-width: 0;
		overflow-y: auto;
		flex: 1 1 auto;
	}

	.head {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}
	.head-fields {
		display: flex;
		gap: var(--spacing-sm);
		align-items: flex-end;
		flex-wrap: wrap;
	}
	.head-fields .id {
		flex: 0 1 12rem;
	}
	.head-fields .name {
		flex: 1 1 16rem;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.field > span {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.field.inline,
	.field.enabled {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.field input[type='text'] {
		width: 100%;
	}
	.field-row {
		display: flex;
		gap: var(--spacing-md);
		flex-wrap: wrap;
	}
	.field-row > .field {
		flex: 1 1 10rem;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		border: none;
		padding: 0;
		margin: 0;
	}
	.block-title,
	.activation legend,
	.advanced summary {
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		color: var(--color-text-primary);
	}
	.activation {
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm, 4px);
		padding: var(--spacing-md);
		gap: var(--spacing-md);
	}
	.advanced summary {
		cursor: pointer;
	}
	.advanced[open] {
		gap: var(--spacing-md);
	}

	.fallback-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.fallback-row {
		display: flex;
		gap: var(--spacing-xs);
		align-items: center;
	}
	.add {
		align-self: flex-start;
		font-size: var(--font-size-sm);
	}
	.remove {
		flex: 0 0 auto;
		line-height: 1;
	}
</style>
