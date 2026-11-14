<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { lorebookStore, type DraftEntry, type SaveState } from '#lib/stores/lorebook.svelte.js';
	import type { EntryMeta, LorebookConfig, Slot } from '#lib/types/lorebook.js';
	import type { ReferenceSource } from '#lib/weaver/editor/weaver-references.js';
	import WeaverField from '#lib/weaver/editor/WeaverField.svelte';
	import StringListField from './StringListField.svelte';
	import SlotPicker from '../SlotPicker.svelte';
	import SaveIndicator from '#lib/components/SaveIndicator.svelte';
	import NumberInput from '#lib/components/NumberInput.svelte';
	import WeaverEditor from '#lib/weaver/editor/WeaverEditor.svelte';
	import FieldShell from '#lib/components/FieldShell.svelte';
	import { settings } from '#lib/stores/settings.svelte.js';

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

	let openAdvanced = $derived(settings.current.autoExpandLorebookAdvanced);

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
				<span>{t.studio.entry.id}</span>
				<input
					type="text"
					value={entry.meta.id}
					oninput={(e) => set('id', e.currentTarget.value)}
				/>
			</label>
			<label class="field name">
				<span>{t.studio.entry.name}</span>
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
				<span>{t.studio.entry.enabled}</span>
			</label>
		</div>
		<SaveIndicator state={saveState} />
	</header>

	<FieldShell label={t.studio.entry.content} tokenText={entry.body}>
		{#snippet editor(fill: boolean)}
			<div class="content-editor" class:fill>
				<WeaverEditor doc={entry.body} onChange={setBody} {references} />
			</div>
		{/snippet}
	</FieldShell>

	<fieldset class="block activation">
		<legend>{t.studio.entry.activation}</legend>

		<div class="field">
			<span>{t.studio.entry.keywords}</span>
			<StringListField
				values={entry.meta.keywords}
				onchange={(v) => set('keywords', v)}
				placeholder={t.studio.entry.keywordPlaceholder}
				ariaLabel={t.studio.entry.keywords}
			/>
		</div>

		<div class="field">
			<span>{t.studio.entry.regex}</span>
			<StringListField
				values={entry.meta.regex}
				onchange={(v) => set('regex', v)}
				placeholder={t.studio.entry.patternPlaceholder}
				ariaLabel={t.studio.entry.regexPatterns}
			/>
		</div>

		<div class="field">
			<span>{t.studio.entry.condition}</span>
			<WeaverField
				doc={entry.meta.condition ?? ''}
				onChange={(v) => setOptStr('condition', v)}
				dialect="expression"
				hint={t.studio.entry.conditionHint}
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
				<span>{t.studio.entry.constant}</span>
			</label>
			<label class="field scan">
				<span>{t.studio.entry.scanDepth}</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.scan_depth ?? ''}
					placeholder={config?.default_scan_depth?.toString() ?? ''}
					onchange={(v) => set('scan_depth', v === '' ? undefined : v)}
					ariaLabel={t.studio.entry.scanDepth}
				/>
			</label>
		</div>
	</fieldset>

	<details class="block advanced" bind:open={openAdvanced}>
		<summary>{t.studio.entry.advanced}</summary>

		<div class="field-row">
			<label class="field">
				<span>{t.studio.entry.priority}</span>
				<NumberInput
					step={1}
					value={entry.meta.priority ?? ''}
					placeholder={config?.default_priority?.toString() ?? ''}
					onchange={(v) => set('priority', v === '' ? undefined : v)}
					ariaLabel={t.studio.entry.priority}
				/>
			</label>
			<label class="field">
				<span>{t.studio.entry.insertionOrder}</span>
				<NumberInput
					step={1}
					value={entry.meta.insertion_order ?? ''}
					onchange={(v) => set('insertion_order', v === '' ? undefined : v)}
					ariaLabel={t.studio.entry.insertionOrder}
				/>
			</label>
		</div>

		<div class="field">
			<span>{t.studio.entry.slot}</span>
			<SlotPicker
				value={entry.meta.slot}
				inheritedDefault={config?.default_slot}
				onchange={(v) => set('slot', v)}
				ariaLabel={t.studio.entry.slot}
			/>
		</div>

		<div class="field">
			<span>{t.studio.entry.fallbackSlots}</span>
			<div class="fallback-list">
				{#each fallback as fb, i (i)}
					<div class="fallback-row">
						<SlotPicker
							value={fb}
							allowInherit={false}
							onchange={(v) => setFallback(i, v)}
							ariaLabel={t.studio.entry.fallback(i + 1)}
						/>
						<button
							type="button"
							class="remove"
							aria-label={t.studio.entry.remove}
							onclick={() => removeFallback(i)}>×</button
						>
					</div>
				{/each}
				<button type="button" class="add" onclick={addFallback}>{t.studio.entry.addFallback}</button
				>
			</div>
		</div>

		<div class="field-row">
			<label class="field">
				<span>{t.studio.entry.stickyTurns}</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.sticky_turns ?? ''}
					onchange={(v) => set('sticky_turns', v === '' ? undefined : v)}
					ariaLabel={t.studio.entry.stickyTurns}
				/>
			</label>
			<label class="field">
				<span>{t.studio.entry.cooldown}</span>
				<NumberInput
					min={0}
					step={1}
					value={entry.meta.cooldown ?? ''}
					onchange={(v) => set('cooldown', v === '' ? undefined : v)}
					ariaLabel={t.studio.entry.cooldown}
				/>
			</label>
		</div>

		<label class="field">
			<span>{t.studio.entry.group}</span>
			<input
				type="text"
				value={entry.meta.group ?? ''}
				oninput={(e) => setOptStr('group', e.currentTarget.value)}
			/>
		</label>

		<div class="field">
			<span>{t.studio.entry.tags}</span>
			<StringListField
				values={entry.meta.tags}
				onchange={(v) => set('tags', v)}
				placeholder={t.studio.entry.tagPlaceholder}
				ariaLabel={t.studio.entry.tags}
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

	.content-editor {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}

	.content-editor.fill {
		flex: 1 1 auto;
		height: 100%;
	}

	.block {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		border: none;
		padding: 0;
		margin: 0;
	}
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
