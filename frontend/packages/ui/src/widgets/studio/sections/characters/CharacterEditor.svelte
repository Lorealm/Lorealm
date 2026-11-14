<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onDestroy } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import AttachmentsSection from './AttachmentsSection.svelte';
	import TagField from './TagField.svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import { dateTimeMedium } from '#lib/assets/scripts/datetime.js';

	const SAVE_MS = 500;

	const draft = $derived(characterStore.editingCharacter);
	const save = $derived(characterStore.saveState);

	let inputEl: HTMLInputElement;
	let busy = $state(false);

	type Section =
		| 'identity'
		| 'persona'
		| 'prompts'
		| 'greetings'
		| 'examples'
		| 'attachments'
		| 'authoring'
		| 'history';
	let section = $state<Section>('identity');

	const sections: Array<{ key: Section; count?: () => number }> = [
		{ key: 'identity' },
		{ key: 'persona' },
		{ key: 'prompts' },
		{ key: 'greetings', count: () => draft?.greetings.length ?? 0 },
		{ key: 'examples', count: () => draft?.examples.length ?? 0 },
		{
			key: 'attachments',
			count: () =>
				(draft?.lorebook_ids.length ?? 0) +
				(draft?.tool_ids.length ?? 0) +
				(draft?.widget_ids.length ?? 0)
		},
		{ key: 'authoring' },
		{ key: 'history', count: () => characterStore.versions.length }
	];

	const currentVersion = $derived(
		characterStore.versions.find((v) => v.id === characterStore.currentVersionId) ?? null
	);

	function formatDate(value: string): string {
		return dateTimeMedium(value) ?? value;
	}

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void characterStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	// The below functions are discrete and do not need debounced saving
	function addGreeting() {
		characterStore.addGreeting();
		requestSave();
	}
	function moveGreeting(i: number, dir: -1 | 1) {
		characterStore.moveGreeting(i, dir);
		requestSave();
	}
	function removeGreeting(i: number) {
		characterStore.removeGreeting(i);
		requestSave();
	}
	function addExample() {
		characterStore.addExample();
		requestSave();
	}
	function moveExample(i: number, dir: -1 | 1) {
		characterStore.moveExample(i, dir);
		requestSave();
	}
	function removeExample(i: number) {
		characterStore.removeExample(i);
		requestSave();
	}
	function openPicker() {
		inputEl.click();
	}

	async function onPick(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		busy = true;
		try {
			await characterStore.changeImage(file);
		} finally {
			busy = false;
			input.value = '';
		}
	}

	// beautiful
	const saveLabel = $derived(
		save.status === 'saving'
			? t.common.saving
			: save.status === 'saved'
				? t.common.saved
				: save.status === 'error'
					? t.studio.characterEditor.unsavedError
					: characterStore.hasUnsavedChanges
						? t.studio.characterEditor.unsaved
						: t.common.saved
	);
</script>

<input
	bind:this={inputEl}
	type="file"
	accept="image/*"
	onchange={onPick}
	hidden
	style="display:none"
/>

{#snippet field(label: string, hint: string, get: () => string, set: (v: string) => void, rows = 6)}
	<ContextField
		{label}
		hint={hint || undefined}
		value={get()}
		{rows}
		onchange={(v) => {
			set(v);
			requestSave();
		}}
	/>
{/snippet}

<div class="character-editor">
	<section class="col-list">
		<header class="list-header">
			<div class="header-row">
				<button
					class="back"
					onclick={() => navigator.start().studio().characters()}
					aria-label={t.studio.characterEditor.back}>←</button
				>
				<span class="char-name" {@attach tooltip(draft?.name)}
					>{draft?.name || t.studio.characterEditor.loading}</span
				>
			</div>
			<div class="save-row" class:error={save.status === 'error'}>
				<span class="dot" data-status={save.status}></span>
				<span>{saveLabel}</span>
			</div>
			{#if currentVersion}
				<div class="version-row">
					<span class="version-chip" class:fresh={characterStore.lastSaveCreatedVersion}
						>v{currentVersion.version_num}</span
					>
					{#if characterStore.lastSaveCreatedVersion}
						<span class="version-note">{t.studio.characterEditor.newVersion}</span>
					{:else}
						<span class="version-note muted">{t.studio.characterEditor.updatingCurrent}</span>
					{/if}
				</div>
			{/if}
			{#if save.status === 'error' && save.message}
				<pre class="save-error">{save.message}</pre>
			{/if}
		</header>

		<nav class="section-nav">
			{#each sections as s (s.key)}
				<button
					class="nav-item"
					class:active={section === s.key}
					type="button"
					onclick={() => (section = s.key)}
				>
					<span>{t.studio.characterEditor.sections[s.key]}</span>
					{#if s.count}<span class="badge">{s.count()}</span>{/if}
				</button>
			{/each}
		</nav>
	</section>

	<section class="col-content">
		{#if draft}
			{#key draft}
				<div class="pane">
					{#if section === 'identity'}
						<h1>{t.studio.characterEditor.sections.identity}</h1>
						<div class="identity-top">
							<div class="avatar">
								{#if characterStore.avatarUrl}
									<img
										src={characterStore.avatarUrl}
										alt={t.studio.characterEditor.avatarAlt(draft.name)}
									/>
								{:else}
									<div class="avatar-placeholder" aria-hidden="true"></div>
								{/if}
								<button onclick={openPicker} disabled={busy} class="pickFile">
									{busy
										? t.studio.characterEditor.uploading
										: t.studio.characterEditor.changeAvatar}</button
								>
							</div>
							<div class="identity-fields">
								<label for="f-name"
									>{t.studio.characterEditor.name}
									<span class="req">{t.studio.characterEditor.nameNote} {`{{char:name}}`}</span
									></label
								>
								<input id="f-name" bind:value={draft.name} oninput={requestSave} />

								<label for="f-display">{t.studio.characterEditor.displayName}</label>
								<input
									id="f-display"
									placeholder={draft.name}
									bind:value={draft.display_name}
									oninput={requestSave}
								/>
							</div>
						</div>

						<TagField
							label={t.studio.characterEditor.tags}
							items={draft.tags}
							onchange={requestSave}
							placeholder={t.studio.characterEditor.addTag}
						/>
						<TagField
							label={t.studio.characterEditor.altNames}
							items={draft.alt_names}
							onchange={requestSave}
							placeholder={t.studio.characterEditor.addName}
						/>
					{:else if section === 'persona'}
						<h1>{t.studio.characterEditor.sections.persona}</h1>
						{@render field(
							t.studio.characterEditor.description,
							t.studio.characterEditor.descriptionHint,
							() => draft.description,
							(v) => (draft.description = v),
							8
						)}
						{@render field(
							t.studio.characterEditor.personality,
							t.studio.characterEditor.personalityHint,
							() => draft.personality,
							(v) => (draft.personality = v)
						)}
						{@render field(
							t.studio.characterEditor.scenario,
							t.studio.characterEditor.scenarioHint,
							() => draft.scenario,
							(v) => (draft.scenario = v)
						)}
					{:else if section === 'prompts'}
						<h1>{t.studio.characterEditor.sections.prompts}</h1>
						{@render field(
							t.studio.characterEditor.systemPrompt,
							t.studio.characterEditor.systemPromptHint,
							() => draft.system_prompt,
							(v) => (draft.system_prompt = v),
							8
						)}
						{@render field(
							t.studio.characterEditor.postHistory,
							t.studio.characterEditor.postHistoryHint,
							() => draft.post_history_instructions,
							(v) => (draft.post_history_instructions = v)
						)}
						{@render field(
							t.studio.characterEditor.injection,
							t.studio.characterEditor.injectionHint,
							() => draft.template_injection,
							(v) => (draft.template_injection = v)
						)}
					{:else if section === 'greetings'}
						<div class="section-head">
							<h1>{t.studio.characterEditor.sections.greetings}</h1>
							<button type="button" class="add" onclick={addGreeting}
								>{t.studio.characterEditor.addGreeting}</button
							>
						</div>
						{#if draft.greetings.length === 0}
							<p class="empty">{t.studio.characterEditor.noGreetings}</p>
						{/if}
						{#each draft.greetings as greeting, i (greeting)}
							<div class="item-card">
								<div class="item-bar">
									<div class="item-actions">
										<button
											type="button"
											aria-label={t.studio.characterEditor.moveUp}
											disabled={i === 0}
											onclick={() => moveGreeting(i, -1)}>↑</button
										>
										<button
											type="button"
											aria-label={t.studio.characterEditor.moveDown}
											disabled={i === draft.greetings.length - 1}
											onclick={() => moveGreeting(i, 1)}>↓</button
										>
										<button
											type="button"
											class="danger"
											aria-label={t.studio.characterEditor.remove}
											onclick={() => removeGreeting(i)}>✕</button
										>
									</div>
								</div>
								<ContextField
									value={greeting.text}
									rows={4}
									onchange={(v) => {
										greeting.text = v;
										requestSave();
									}}
									label={t.studio.characterEditor.greeting(i + 1)}
								/>
							</div>
						{/each}
					{:else if section === 'examples'}
						<div class="section-head">
							<h1>{t.studio.characterEditor.exampleDialogues}</h1>
							<button type="button" class="add" onclick={addExample}
								>{t.studio.characterEditor.addExample}</button
							>
						</div>
						{#if draft.examples.length === 0}
							<p class="empty">{t.studio.characterEditor.noExamples}</p>
						{/if}
						{#each draft.examples as example, i (example)}
							<div class="item-card">
								<div class="item-bar">
									<div class="item-actions">
										<button
											type="button"
											aria-label={t.studio.characterEditor.moveUp}
											disabled={i === 0}
											onclick={() => moveExample(i, -1)}>↑</button
										>
										<button
											type="button"
											aria-label={t.studio.characterEditor.moveDown}
											disabled={i === draft.examples.length - 1}
											onclick={() => moveExample(i, 1)}>↓</button
										>
										<button
											type="button"
											class="danger"
											aria-label={t.studio.characterEditor.remove}
											onclick={() => removeExample(i)}>✕</button
										>
									</div>
								</div>
								<ContextField
									label={t.studio.characterEditor.example(i + 1)}
									value={example.content}
									rows={5}
									onchange={(v) => {
										example.content = v;
										requestSave();
									}}
								/>
							</div>
						{/each}
					{:else if section === 'attachments'}
						<AttachmentsSection onchange={requestSave} />
					{:else if section === 'authoring'}
						<h1>{t.studio.characterEditor.sections.authoring}</h1>
						<label for="f-creator">{t.studio.characterEditor.creator}</label>
						<input id="f-creator" bind:value={draft.creator} oninput={requestSave} />

						<label for="f-version">{t.studio.characterEditor.cardVersion}</label>
						<input id="f-version" bind:value={draft.card_version} oninput={requestSave} />

						<ContextField
							label={t.studio.characterEditor.creatorNotes}
							hint={t.studio.characterEditor.creatorNotesHint}
							value={draft.creator_notes}
							rows={6}
							tokens={false}
							weaver={false}
							onchange={(v) => {
								draft.creator_notes = v;
								requestSave();
							}}
						/>
					{:else if section === 'history'}
						<h1>{t.studio.characterEditor.sections.history}</h1>
						<p class="hint">
							{t.studio.characterEditor.historyHint}
						</p>
						{#if characterStore.versions.length === 0}
							<p class="empty">{t.studio.characterEditor.noVersions}</p>
						{/if}
						{#each characterStore.versions as v (v.id)}
							<div class="version-item" class:current={v.id === characterStore.currentVersionId}>
								<div class="version-item-main">
									<span class="version-item-num">v{v.version_num}</span>
									<span class="version-item-card">{v.card_version}</span>
									{#if v.id === characterStore.currentVersionId}
										<span class="version-item-tag">{t.studio.characterEditor.current}</span>
									{/if}
								</div>
								<span class="version-item-date">{formatDate(v.created_at)}</span>
							</div>
						{/each}
					{/if}
				</div>
			{/key}
		{/if}
	</section>
</div>

<style>
	.character-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
		position: relative;
	}
	.col-list {
		flex: 0 0 240px;
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		border-right: 1px solid var(--color-border-subtle);
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
		overflow-y: auto;
	}
	.list-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.header-row {
		display: flex;
		flex: 0 1 auto;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.back {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.char-name {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.save-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.save-row.error {
		color: var(--color-error-text);
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background-color: var(--color-text-muted);
	}
	.dot[data-status='saving'] {
		background-color: var(--color-warning);
	}
	.dot[data-status='saved'] {
		background-color: var(--color-success);
	}
	.dot[data-status='error'] {
		background-color: var(--color-error);
	}
	.save-error {
		margin: 0;
		max-height: 120px;
		overflow: auto;
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
		border: 1px solid var(--color-error-border);
		border-radius: var(--radius-sm);
		padding: var(--spacing-sm);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		white-space: pre-wrap;
	}
	.version-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.version-chip {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		border-radius: var(--radius-full);
		padding: 0 var(--spacing-sm);
		font-size: var(--font-size-xs);
	}
	.version-chip.fresh {
		background-color: var(--color-success-bg);
		color: var(--color-success-text);
	}
	.version-note {
		color: var(--color-success-text);
		font-size: var(--font-size-xs);
	}
	.version-note.muted {
		color: var(--color-text-muted);
	}
	.version-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm) var(--spacing-md);
	}
	.version-item.current {
		border-color: var(--color-border-active);
	}
	.version-item-main {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.version-item-num {
		color: var(--color-text-primary);
		font-weight: var(--font-weight-semibold);
	}
	.version-item-card {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.version-item-tag {
		background-color: var(--color-selected);
		color: var(--color-text-accent);
		border-radius: var(--radius-full);
		padding: 0 var(--spacing-sm);
		font-size: var(--font-size-xs);
	}
	.version-item-date {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.section-nav {
		display: flex;
		flex-direction: column;
		padding: var(--spacing-sm);
		gap: var(--spacing-xs);
		overflow-y: auto;
	}
	.nav-item {
		justify-content: space-between;
		align-items: center;
		background: transparent;
		font-size: var(--font-size-sm);
		padding: var(--spacing-sm);
	}
	.nav-item:hover {
		background-color: var(--color-hover);
	}
	.nav-item.active {
		background-color: var(--color-selected);
		color: var(--color-text-primary);
		font-weight: var(--font-weight-semibold);
	}
	.badge {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		border-radius: var(--radius-full);
		padding: 0 var(--spacing-sm);
		font-size: var(--font-size-xs);
	}
	.pane {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
		padding: var(--spacing-xl);
	}
	.pane h1 {
		font-size: var(--font-size-h3);
		margin: 0;
	}
	.section-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.add {
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.identity-top {
		display: flex;
		gap: var(--spacing-lg);
		align-items: flex-start;
	}
	.avatar {
		flex: 0 0 auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		width: 120px;
	}
	.avatar img,
	.avatar-placeholder {
		width: 120px;
		aspect-ratio: var(--avatar-aspect-ratio);
		border-radius: var(--radius-lg);
		object-fit: cover;
		box-shadow: var(--shadow-md);
	}
	.avatar-placeholder {
		background: var(--gradient-portrait);
	}
	.pickFile {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.identity-fields {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.req {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		margin: 0;
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.item-card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm);
	}
	.item-bar {
		display: flex;
		align-items: center;
		justify-content: flex-end;
	}
	.item-actions {
		display: flex;
		gap: var(--spacing-xs);
	}
	.item-actions button {
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.item-actions button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.item-actions .danger:hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	label {
		margin-top: var(--spacing-xs);
	}

	@media (max-width: 768px) {
		.character-editor {
			flex-direction: column;
		}
		.col-list {
			flex: 0 0 auto;
			border-right: none;
			border-bottom: 1px solid var(--color-border-subtle);
		}
		.list-header {
			padding: var(--spacing-sm) var(--spacing-md);
		}
		.section-nav {
			flex-direction: row;
			overflow-x: auto;
			overflow-y: hidden;
			scrollbar-width: none;
		}
		.nav-item {
			flex-shrink: 0;
			gap: var(--spacing-xs);
			white-space: nowrap;
		}
		.pane {
			padding: var(--spacing-md);
		}
		.identity-top {
			flex-direction: column;
			align-items: stretch;
		}
		.avatar {
			align-self: center;
		}
	}
</style>
