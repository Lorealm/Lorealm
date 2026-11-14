<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../../stores/tooltip.svelte.ts';
	import type { LorebookDraft, SaveState } from '#lib/stores/lorebook.svelte.js';
	import type { NamespaceAccess, NamespaceConfig, Slot } from '#lib/types/lorebook.js';
	import SlotPicker from '../SlotPicker.svelte';
	import SaveIndicator from '#lib/components/SaveIndicator.svelte';
	import NumberInput from '#lib/components/NumberInput.svelte';

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
	let newNamespace = $state('');
	let newAccess = $state<NamespaceAccess>('readonly');
	let newDescription = $state('');
	function setNamespace(key: string, ns: NamespaceConfig) {
		draft.config.namespaces = { ...config.namespaces, [key]: ns };
		touch();
	}
	function setNamespaceAccess(key: string, access: NamespaceAccess) {
		setNamespace(key, { ...config.namespaces[key], access });
	}
	function setNamespaceDescription(key: string, description: string) {
		setNamespace(key, { ...config.namespaces[key], description });
	}
	function removeNamespace(key: string) {
		const next = { ...config.namespaces };
		delete next[key];
		draft.config.namespaces = next;
		touch();
	}
	function addNamespace() {
		const key = newNamespace.trim();
		if (key === '') return;
		setNamespace(key, { access: newAccess, description: newDescription.trim() });
		newNamespace = '';
		newAccess = 'readonly';
		newDescription = '';
	}
</script>

<div class="config-editor">
	<header class="head">
		<h2>{t.studio.lorebookConfig.title}</h2>
		<SaveIndicator state={saveState} />
	</header>

	<label class="field">
		<span>{t.studio.lorebookConfig.name}</span>
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
		<span>{t.studio.lorebookConfig.description}</span>
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
		<legend>{t.studio.lorebookConfig.defaults}</legend>
		<div class="field-row">
			<label class="field">
				<span>{t.studio.lorebookConfig.scanDepth}</span>
				<NumberInput
					min={0}
					step={1}
					value={config.default_scan_depth}
					onchange={(v) => {
						draft.config.default_scan_depth = v === '' ? 0 : v;
						touch();
					}}
					ariaLabel={t.studio.lorebookConfig.defaultScanDepth}
				/>
			</label>
			<label class="field">
				<span>{t.studio.lorebookConfig.priority}</span>
				<NumberInput
					step={1}
					value={config.default_priority}
					onchange={(v) => {
						draft.config.default_priority = v === '' ? 0 : v;
						touch();
					}}
					ariaLabel={t.studio.lorebookConfig.defaultPriority}
				/>
			</label>
		</div>
		<div class="field">
			<span>{t.studio.lorebookConfig.slot}</span>
			<SlotPicker
				value={config.default_slot}
				allowInherit={false}
				onchange={setDefaultSlot}
				ariaLabel={t.studio.lorebookConfig.defaultSlot}
			/>
		</div>
	</fieldset>

	<fieldset class="block">
		<legend>{t.studio.lorebookConfig.budgets}</legend>
		<label class="field">
			<span>{t.studio.lorebookConfig.tokenBudget}</span>
			<NumberInput
				min={0}
				step={1}
				placeholder={t.studio.lorebookConfig.unlimited}
				value={config.token_budget ?? ''}
				onchange={(v) => {
					draft.config.token_budget = v === '' ? null : v;
					touch();
				}}
				ariaLabel={t.studio.lorebookConfig.tokenBudget}
			/>
		</label>

		<div class="field">
			<span>{t.studio.lorebookConfig.groupBudgets}</span>
			<div class="map-list">
				{#each groupRows as [key, value] (key)}
					<div class="map-row">
						<span class="map-key" {@attach tooltip(key)}>{key}</span>
						<NumberInput
							min={0}
							step={1}
							width="7rem"
							{value}
							onchange={(v) => setGroupBudget(key, v === '' ? 0 : v)}
							ariaLabel={t.studio.lorebookConfig.groupBudget(key)}
						/>
						<button
							type="button"
							class="remove"
							aria-label={t.studio.lorebookConfig.remove}
							onclick={() => removeGroup(key)}>×</button
						>
					</div>
				{/each}
				<div class="map-row add">
					<input
						type="text"
						placeholder={t.studio.lorebookConfig.groupName}
						bind:value={newGroup}
					/>
					<NumberInput
						min={0}
						step={1}
						width="7rem"
						placeholder={t.studio.lorebookConfig.budget}
						value={newBudget}
						onchange={(v) => (newBudget = v)}
						ariaLabel={t.studio.lorebookConfig.newGroupBudget}
					/>
					<button type="button" onclick={addGroup}>{t.studio.lorebookConfig.add}</button>
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
		<span>{t.studio.lorebookConfig.caseSensitive}</span>
	</label>

	<fieldset class="block">
		<legend>{t.studio.lorebookConfig.namespaces}</legend>
		<div class="map-list">
			{#each namespaceRows as [name, ns] (name)}
				<div class="ns-row">
					<code class="ns-name" {@attach tooltip(name)}>{name}</code>
					<select
						class="ns-access"
						value={ns.access}
						onchange={(e) => setNamespaceAccess(name, e.currentTarget.value as NamespaceAccess)}
						aria-label={t.studio.lorebookConfig.access(name)}
					>
						<option value="readonly">{t.studio.lorebookConfig.readonly}</option>
						<option value="readwrite">{t.studio.lorebookConfig.readwrite}</option>
					</select>
					<input
						type="text"
						class="ns-desc"
						placeholder={t.studio.lorebookConfig.descriptionPlaceholder}
						value={ns.description}
						oninput={(e) => setNamespaceDescription(name, e.currentTarget.value)}
						aria-label={t.studio.lorebookConfig.descriptionAria(name)}
					/>
					<button
						type="button"
						class="remove"
						aria-label={t.studio.lorebookConfig.remove}
						onclick={() => removeNamespace(name)}>×</button
					>
				</div>
			{/each}
			<div class="ns-row add">
				<input
					type="text"
					class="ns-name-input"
					placeholder={t.studio.lorebookConfig.namespacePlaceholder}
					bind:value={newNamespace}
				/>
				<select
					class="ns-access"
					bind:value={newAccess}
					aria-label={t.studio.lorebookConfig.newAccess}
				>
					<option value="readonly">{t.studio.lorebookConfig.readonly}</option>
					<option value="readwrite">{t.studio.lorebookConfig.readwrite}</option>
				</select>
				<input
					type="text"
					class="ns-desc"
					placeholder={t.studio.lorebookConfig.descriptionPlaceholder}
					bind:value={newDescription}
				/>
				<button type="button" onclick={addNamespace}>{t.studio.lorebookConfig.add}</button>
			</div>
		</div>
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
	.ns-row {
		display: flex;
		gap: var(--spacing-xs);
		align-items: center;
	}
	.ns-name {
		flex: 0 1 16em;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-family: var(--font-family-mono, ui-monospace, monospace);
		font-size: var(--font-size-sm);
	}
	.ns-access {
		flex: 0 1 auto;
	}
	.ns-desc {
		flex: 1 1 auto;
		min-width: 0;
	}
	.ns-row.add .ns-name-input {
		flex: 0 1 16rem;
		min-width: 0;
	}
</style>
