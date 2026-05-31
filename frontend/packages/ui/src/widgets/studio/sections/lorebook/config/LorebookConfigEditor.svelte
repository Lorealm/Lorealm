<script lang="ts">
	import type { LorebookDraft, SaveState } from '$lib/stores/lorebook.svelte.js';
	import type { Slot } from '$lib/types/lorebook.js';
	import SlotPicker from '../SlotPicker.svelte';
	import SaveIndicator from '../SaveIndicator.svelte';
	import NumberInput from '$lib/components/NumberInput.svelte';

	// The lorebook's global settings. Edits mutate the draft (reactive); the
	// shell debounces `requestSave` into `lorebookStore.saveLorebook`. Note: the
	// visible Name/Description bind to the top-level fields; `config.name` /
	// `config.description` (which also exist in lorebook.yaml) are passed through
	// untouched on save.
	let {
		draft,
		saveState,
		requestSave
	}: {
		draft: LorebookDraft;
		saveState: SaveState;
		requestSave: () => void;
	} = $props();

	const config = $derived(draft.config);

	function touch() {
		requestSave();
	}
	function setDefaultSlot(v: Slot | undefined) {
		draft.config.default_slot = v ?? 'backdrop'; // the default must be concrete
		touch();
	}

	// group budgets: edit values / remove in place; add via a dedicated form so
	// the record never has to hold a transient empty key.
	const groupRows = $derived(Object.entries(config.group_budgets ?? {}));
	let newGroup = $state('');
	let newBudget = $state<number | ''>('');
	function setGroupBudget(key: string, n: number) {
		draft.config.group_budgets = { ...config.group_budgets, [key]: n };
		touch();
	}
	function removeGroup(key: string) {
		const next = { ...config.group_budgets };
		delete next[key];
		draft.config.group_budgets = next;
		touch();
	}
	function addGroup() {
		const key = newGroup.trim();
		if (key === '') return;
		setGroupBudget(key, newBudget === '' ? 0 : newBudget);
		newGroup = '';
		newBudget = '';
	}

	const namespaceRows = $derived(Object.entries(config.namespaces ?? {}));
</script>

<div class="config-editor">
	<header class="head">
		<h2>Lorebook settings</h2>
		<SaveIndicator state={saveState} />
	</header>

	<label class="field">
		<span>Name</span>
		<input
			type="text"
			value={draft.name}
			oninput={(e) => {
				draft.name = e.currentTarget.value;
				touch();
			}}
		/>
	</label>

	<label class="field">
		<span>Description</span>
		<textarea
			rows="3"
			value={draft.description}
			oninput={(e) => {
				draft.description = e.currentTarget.value;
				touch();
			}}
		></textarea>
	</label>

	<fieldset class="block">
		<legend>Defaults</legend>
		<div class="field-row">
			<label class="field">
				<span>Scan depth</span>
				<NumberInput
					min={0}
					step={1}
					value={config.default_scan_depth}
					onchange={(v) => {
						draft.config.default_scan_depth = v === '' ? 0 : v;
						touch();
					}}
					ariaLabel="Default scan depth"
				/>
			</label>
			<label class="field">
				<span>Priority</span>
				<NumberInput
					step={1}
					value={config.default_priority}
					onchange={(v) => {
						draft.config.default_priority = v === '' ? 0 : v;
						touch();
					}}
					ariaLabel="Default priority"
				/>
			</label>
		</div>
		<div class="field">
			<span>Slot</span>
			<SlotPicker
				value={config.default_slot}
				allowInherit={false}
				onchange={setDefaultSlot}
				ariaLabel="Default slot"
			/>
		</div>
	</fieldset>

	<fieldset class="block">
		<legend>Budgets</legend>
		<label class="field">
			<span>Token budget</span>
			<NumberInput
				min={0}
				step={1}
				placeholder="unlimited"
				value={config.token_budget ?? ''}
				onchange={(v) => {
					draft.config.token_budget = v === '' ? null : v;
					touch();
				}}
				ariaLabel="Token budget"
			/>
		</label>

		<div class="field">
			<span>Group budgets</span>
			<div class="map-list">
				{#each groupRows as [key, value] (key)}
					<div class="map-row">
						<span class="map-key" title={key}>{key}</span>
						<NumberInput
							min={0}
							step={1}
							width="7rem"
							{value}
							onchange={(v) => setGroupBudget(key, v === '' ? 0 : v)}
							ariaLabel={`${key} budget`}
						/>
						<button
							type="button"
							class="remove"
							aria-label="Remove"
							onclick={() => removeGroup(key)}>×</button
						>
					</div>
				{/each}
				<div class="map-row add">
					<input type="text" placeholder="group name" bind:value={newGroup} />
					<NumberInput
						min={0}
						step={1}
						width="7rem"
						placeholder="budget"
						value={newBudget}
						onchange={(v) => (newBudget = v)}
						ariaLabel="New group budget"
					/>
					<button type="button" onclick={addGroup}>+ Add</button>
				</div>
			</div>
		</div>
	</fieldset>

	<label class="field inline">
		<input
			type="checkbox"
			checked={config.case_sensitive_keywords}
			onchange={(e) => {
				draft.config.case_sensitive_keywords = e.currentTarget.checked;
				touch();
			}}
		/>
		<span>Case-sensitive keywords</span>
	</label>

	<fieldset class="block">
		<legend>Namespaces</legend>
		{#if namespaceRows.length === 0}
			<p class="muted">None declared.</p>
		{:else}
			<ul class="namespaces">
				{#each namespaceRows as [name, ns] (name)}
					<li>
						<code>{name}</code>
						<span class="access">{ns.access}</span>
						{#if ns.description}<span class="muted">— {ns.description}</span>{/if}
					</li>
				{/each}
			</ul>
		{/if}
		<p class="muted hint">Namespaces are read-only here.</p>
	</fieldset>
</div>

<style>
	code {
		color: var(--color-info);
	}
	.config-editor {
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
		align-items: baseline;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.head h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		color: var(--color-text-primary);
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
	.field input[type='text'],
	.field textarea {
		width: 100%;
	}
	.field.inline {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
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
		gap: var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm, 4px);
		padding: var(--spacing-md);
		margin: 0;
	}
	.block legend {
		font-family: var(--font-family-heading);
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
		padding: 0 var(--spacing-xs);
	}
	.map-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.map-row {
		display: flex;
		gap: var(--spacing-xs);
		align-items: center;
	}
	.map-row .map-key {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--font-family-mono, ui-monospace, monospace);
		font-size: var(--font-size-sm);
	}
	.map-row.add input[type='text'] {
		flex: 1 1 auto;
		min-width: 0;
	}
	.remove {
		flex: 0 0 auto;
		line-height: 1;
	}
	.namespaces {
		margin: 0;
		padding-left: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
	}
	.access {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.muted {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		margin: 0;
	}
	.hint {
		margin-top: calc(-1 * var(--spacing-xs));
	}
</style>
