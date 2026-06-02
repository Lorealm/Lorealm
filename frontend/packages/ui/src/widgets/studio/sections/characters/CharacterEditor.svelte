<script lang="ts">
	import { onDestroy, untrack } from 'svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte';
	import type { Snowflake } from '$lib/types/core.js';
	import TagField from './TagField.svelte';

	const SAVE_MS = 500;

	let { id }: { id: Snowflake } = $props();

	const draft = $derived(characterStore.editingCharacter);
	const save = $derived(characterStore.saveState);

	type Section =
		| 'identity'
		| 'persona'
		| 'prompts'
		| 'greetings'
		| 'examples'
		| 'authoring'
		| 'history';
	let section = $state<Section>('identity');

	const sections: Array<{ key: Section; label: string; count?: () => number }> = [
		{ key: 'identity', label: 'Identity' },
		{ key: 'persona', label: 'Persona' },
		{ key: 'prompts', label: 'Prompts' },
		{ key: 'greetings', label: 'Greetings', count: () => draft?.greetings.length ?? 0 },
		{ key: 'examples', label: 'Examples', count: () => draft?.examples.length ?? 0 },
		{ key: 'authoring', label: 'Authoring' },
		{ key: 'history', label: 'History', count: () => characterStore.versions.length }
	];

	// The version number edits are currently landing in, looked up from history.
	const currentVersion = $derived(
		characterStore.versions.find((v) => v.id === characterStore.currentVersionId) ?? null
	);

	function formatDate(value: unknown): string {
		if (value && typeof (value as { toLocaleString?: unknown }).toLocaleString === 'function') {
			return (value as { toLocaleString: () => string }).toLocaleString();
		}
		return String(value ?? '');
	}

	// (Re)open the editing session whenever the routed id changes. Untracked so
	// the synchronous store reads don't subscribe this effect.
	$effect(() => {
		const cid = id;
		untrack(() => {
			section = 'identity';
			maximized = null;
			void characterStore.startEditing(cid);
		});
	});
	onDestroy(() => characterStore.stopEditing());

	// --- debounced autosave (one save covers the whole character) -----------
	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void characterStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	// Discrete list actions mutate the draft AND schedule a save — without this
	// a reorder/add/remove wouldn't persist until the next keystroke in a field.
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

	// --- maximize a single text field ---------------------------------------
	// The overlay edits a local copy (real two-way bind, so no cursor jank) and
	// propagates each change back to the draft through the field's setter, so
	// autosave is unchanged. It's rendered inside .character-editor and
	// absolutely positioned, so it fills the editor's own box — not the viewport.
	let maximizedValue = $state('');
	let maximized = $state<{ title: string; set: (v: string) => void } | null>(null);
	function maximize(title: string, get: () => string, set: (v: string) => void) {
		maximizedValue = get();
		maximized = { title, set };
	}
	function minimize() {
		maximized = null;
	}
	function autofocus(node: HTMLTextAreaElement) {
		node.focus();
	}

	const saveLabel = $derived(
		save.status === 'saving'
			? 'Saving…'
			: save.status === 'saved'
				? 'Saved'
				: save.status === 'error'
					? 'Unsaved — error'
					: characterStore.hasUnsavedChanges
						? 'Unsaved'
						: 'Saved'
	);
</script>

{#snippet maxBtn(title: string, get: () => string, set: (v: string) => void)}
	<button
		type="button"
		class="maximize"
		title="Maximize"
		aria-label={`Maximize ${title}`}
		onclick={() => maximize(title, get, set)}>⤢</button
	>
{/snippet}

<div class="character-editor">
	<section class="col-list">
		<header class="list-header">
			<div class="header-row">
				<button class="back" onclick={() => studioStore.close()} aria-label="Back to characters"
					>←</button
				>
				<span class="char-name" title={draft?.name}>{draft?.name || 'Loading…'}</span>
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
						<span class="version-note">new version started</span>
					{:else}
						<span class="version-note muted">updating current</span>
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
					<span>{s.label}</span>
					{#if s.count}<span class="badge">{s.count()}</span>{/if}
				</button>
			{/each}
		</nav>
	</section>

	<section class="col-content">
		{#if draft}
			<div class="pane">
				{#if section === 'identity'}
					<h1>Identity</h1>
					<div class="identity-top">
						<div class="avatar">
							{#if characterStore.avatarUrl}
								<img src={characterStore.avatarUrl} alt={`${draft.name} avatar`} />
							{:else}
								<div class="avatar-placeholder" aria-hidden="true"></div>
							{/if}
							<p class="hint">Avatar is managed separately and can't be changed here yet.</p>
						</div>
						<div class="identity-fields">
							<label for="f-name">Name <span class="req">in-chat, {`{{char}}`}</span></label>
							<input id="f-name" bind:value={draft.name} oninput={requestSave} />

							<label for="f-display">Display name</label>
							<input
								id="f-display"
								placeholder={draft.name}
								bind:value={draft.display_name}
								oninput={requestSave}
							/>
						</div>
					</div>

					<TagField label="Tags" items={draft.tags} onchange={requestSave} placeholder="Add tag…" />
					<TagField
						label="Alternate names"
						items={draft.alt_names}
						onchange={requestSave}
						placeholder="Add name…"
					/>
				{:else if section === 'persona'}
					<h1>Persona</h1>
					<label for="f-desc">Description</label>
					<div class="expandable">
						<textarea id="f-desc" rows="8" bind:value={draft.description} oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'Description',
							() => draft.description,
							(v) => (draft.description = v)
						)}
					</div>

					<label for="f-pers">Personality</label>
					<div class="expandable">
						<textarea id="f-pers" rows="6" bind:value={draft.personality} oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'Personality',
							() => draft.personality,
							(v) => (draft.personality = v)
						)}
					</div>

					<label for="f-scen">Scenario</label>
					<div class="expandable">
						<textarea id="f-scen" rows="6" bind:value={draft.scenario} oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'Scenario',
							() => draft.scenario,
							(v) => (draft.scenario = v)
						)}
					</div>
				{:else if section === 'prompts'}
					<h1>Prompts</h1>
					<label for="f-sys">System prompt</label>
					<div class="expandable">
						<textarea id="f-sys" rows="8" bind:value={draft.system_prompt} oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'System prompt',
							() => draft.system_prompt,
							(v) => (draft.system_prompt = v)
						)}
					</div>

					<label for="f-phi">Post-history instructions</label>
					<div class="expandable">
						<textarea
							id="f-phi"
							rows="6"
							bind:value={draft.post_history_instructions}
							oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'Post-history instructions',
							() => draft.post_history_instructions,
							(v) => (draft.post_history_instructions = v)
						)}
					</div>
				{:else if section === 'greetings'}
					<div class="section-head">
						<h1>Greetings</h1>
						<button type="button" class="add" onclick={addGreeting}>＋ Greeting</button>
					</div>
					{#if draft.greetings.length === 0}
						<p class="empty">No greetings yet. The first one is the default opener.</p>
					{/if}
					{#each draft.greetings as greeting, i (greeting)}
						<div class="item-card">
							<div class="item-bar">
								<span class="item-label">Greeting {i + 1}</span>
								<div class="item-actions">
									<button
										type="button"
										aria-label="Move up"
										disabled={i === 0}
										onclick={() => moveGreeting(i, -1)}>↑</button
									>
									<button
										type="button"
										aria-label="Move down"
										disabled={i === draft.greetings.length - 1}
										onclick={() => moveGreeting(i, 1)}>↓</button
									>
									<button
										type="button"
										class="danger"
										aria-label="Remove"
										onclick={() => removeGreeting(i)}>✕</button
									>
								</div>
							</div>
							<div class="expandable">
								<textarea rows="4" bind:value={greeting.text} oninput={requestSave}></textarea>
								{@render maxBtn(
									`Greeting ${i + 1}`,
									() => greeting.text,
									(v) => (greeting.text = v)
								)}
							</div>
						</div>
					{/each}
				{:else if section === 'examples'}
					<div class="section-head">
						<h1>Example dialogues</h1>
						<button type="button" class="add" onclick={addExample}>＋ Example</button>
					</div>
					{#if draft.examples.length === 0}
						<p class="empty">No example dialogues yet.</p>
					{/if}
					{#each draft.examples as example, i (example)}
						<div class="item-card">
							<div class="item-bar">
								<span class="item-label">Example {i + 1}</span>
								<div class="item-actions">
									<button
										type="button"
										aria-label="Move up"
										disabled={i === 0}
										onclick={() => moveExample(i, -1)}>↑</button
									>
									<button
										type="button"
										aria-label="Move down"
										disabled={i === draft.examples.length - 1}
										onclick={() => moveExample(i, 1)}>↓</button
									>
									<button
										type="button"
										class="danger"
										aria-label="Remove"
										onclick={() => removeExample(i)}>✕</button
									>
								</div>
							</div>
							<div class="expandable">
								<textarea rows="5" bind:value={example.content} oninput={requestSave}></textarea>
								{@render maxBtn(
									`Example ${i + 1}`,
									() => example.content,
									(v) => (example.content = v)
								)}
							</div>
						</div>
					{/each}
				{:else if section === 'authoring'}
					<h1>Authoring</h1>
					<label for="f-creator">Creator</label>
					<input id="f-creator" bind:value={draft.creator} oninput={requestSave} />

					<label for="f-version">Card version</label>
					<input id="f-version" bind:value={draft.card_version} oninput={requestSave} />

					<label for="f-notes">Creator notes</label>
					<div class="expandable">
						<textarea id="f-notes" rows="6" bind:value={draft.creator_notes} oninput={requestSave}
						></textarea>
						{@render maxBtn(
							'Creator notes',
							() => draft.creator_notes,
							(v) => (draft.creator_notes = v)
						)}
					</div>

					{#if characterStore.lorebookId}
						<p class="hint">Linked lorebook: {characterStore.lorebookId}</p>
					{/if}
				{:else if section === 'history'}
					<h1>History</h1>
					<p class="hint">
						Edits made close together are grouped into one version. A new version starts after a
						period of inactivity.
					</p>
					{#if characterStore.versions.length === 0}
						<p class="empty">No versions recorded yet.</p>
					{/if}
					{#each characterStore.versions as v (v.id)}
						<div class="version-item" class:current={v.id === characterStore.currentVersionId}>
							<div class="version-item-main">
								<span class="version-item-num">v{v.version_num}</span>
								<span class="version-item-card">{v.card_version}</span>
								{#if v.id === characterStore.currentVersionId}
									<span class="version-item-tag">current</span>
								{/if}
							</div>
							<span class="version-item-date">{formatDate(v.created_at)}</span>
						</div>
					{/each}
				{/if}
			</div>
		{/if}
	</section>

	{#if maximized}
		<div class="maximize-overlay">
			<div class="maximize-bar">
				<span class="maximize-title">{maximized.title}</span>
				<button
					type="button"
					class="maximize-close"
					title="Minimize"
					aria-label="Minimize"
					onclick={minimize}>⤡ Minimize</button
				>
			</div>
			<textarea
				class="maximize-area"
				use:autofocus
				bind:value={maximizedValue}
				oninput={() => {
					maximized?.set(maximizedValue);
					requestSave();
				}}
				onkeydown={(e) => {
					if (e.key === 'Escape') minimize();
				}}
			></textarea>
		</div>
	{/if}
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
		align-items: center;
		gap: var(--spacing-sm);
	}
	.back {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.char-name {
		flex: 1 1 auto;
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
		height: 120px;
		border-radius: var(--radius-lg);
		object-fit: cover;
		box-shadow: var(--shadow-md);
	}
	.avatar-placeholder {
		background: var(--gradient-portrait);
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
		justify-content: space-between;
	}
	.item-label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
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

	/* --- expandable text fields --- */
	.expandable {
		position: relative;
	}
	.maximize {
		position: absolute;
		top: var(--spacing-xs);
		right: var(--spacing-sm);
		padding: 2px 6px;
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		border-radius: var(--radius-sm);
		font-size: var(--font-size-sm);
		line-height: 1;
		opacity: 0;
		transition: opacity var(--transition-fast) var(--easing-default);
	}
	.expandable:hover .maximize,
	.expandable:focus-within .maximize,
	.maximize:focus-visible {
		opacity: 0.7;
	}
	.maximize:hover {
		opacity: 1;
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}

	/* --- maximize overlay (fills the editor's own box, not the viewport) --- */
	.maximize-overlay {
		position: absolute;
		inset: 0;
		z-index: var(--z-index-elevated);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		background-color: var(--color-bg-base);
	}
	.maximize-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex: 0 0 auto;
	}
	.maximize-title {
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.maximize-close {
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.maximize-area {
		flex: 1 1 auto;
		min-height: 0;
		resize: none;
	}
</style>
