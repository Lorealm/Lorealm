<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { personaStore } from '$lib/stores/persona.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import TransportImage from '$lib/components/TransportImage.svelte';

	const SAVE_MS = 500;

	const draft = $derived(personaStore.editingPersona);
	const save = $derived(personaStore.saveState);
	const personas = $derived(personaStore.loadedPersonas ?? []);
	const selectedId = $derived(personaStore.editingId);
	const activePersonaId = $derived(personaStore.activePersonaId);

	// Same path as the avatar PUT, GET-side. The version query busts the cache
	// for the persona being edited so TransportImage refetches after an upload.
	function avatarSrc(id: Snowflake): string {
		const bust =
			id === personaStore.editingId && personaStore.avatarVersion
				? `?v=${personaStore.avatarVersion}`
				: '';
		return `/api/v1/persona/${id}/avatar${bust}`;
	}

	let inputEl: HTMLInputElement;
	let busy = $state(false);
	let avatarError = $state<string | null>(null);

	onMount(() => {
		void personaStore.loadPersonas();
		void personaStore.loadActive();
	});
	onDestroy(() => personaStore.stopEditing());

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void personaStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function select(id: Snowflake) {
		if (id === personaStore.editingId) return;
		clearTimeout(timer);
		await personaStore.save(); // flush any pending edits before switching (no-op if none)
		void personaStore.startEditing(id);
	}

	function openPicker() {
		inputEl.click();
	}

	async function makeActive(id: Snowflake) {
		try {
			await personaStore.setActive(id);
		} catch {
			// The optimistic update is reverted in the store; nothing more to do.
		}
	}

	async function onPick(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		avatarError = null;
		busy = true;
		try {
			await personaStore.setAvatar(file);
		} catch (err) {
			avatarError = err instanceof Error ? err.message : 'Upload failed';
		} finally {
			busy = false;
			input.value = '';
		}
	}

	const saveLabel = $derived(
		save.status === 'saving'
			? 'Saving…'
			: save.status === 'error'
				? 'Unsaved; error'
				: personaStore.hasUnsavedChanges
					? 'Unsaved'
					: 'Saved'
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

{#snippet hint(text: string)}
	<button type="button" class="hint" title={text} aria-label={`Help: ${text}`}>?</button>
{/snippet}

<div class="persona-editor">
	<section class="col-list">
		<header class="list-header">
			<span class="title">Personas</span>
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span>{saveLabel}</span>
				</div>
			{/if}
			{#if save.status === 'error' && save.message}
				<pre class="save-error">{save.message}</pre>
			{/if}
		</header>

		<nav class="persona-list">
			{#if personas.length === 0}
				<p class="empty">No personas yet.</p>
			{/if}
			{#each personas as persona (persona.id)}
				<div
					class="persona-item"
					class:selected={persona.id === selectedId}
					class:is-active={persona.id === activePersonaId}
				>
					<button class="persona-open" type="button" onclick={() => select(persona.id)}>
						<span class="list-avatar">
							<TransportImage src={avatarSrc(persona.id)} alt="" />
						</span>
						<span class="persona-text">
							<span class="persona-name">{persona.name || 'Untitled'}</span>
							{#if persona.display_name}
								<span class="persona-sub">{persona.display_name}</span>
							{/if}
						</span>
					</button>
					<button
						class="active-toggle"
						class:on={persona.id === activePersonaId}
						type="button"
						title={persona.id === activePersonaId ? 'Active persona' : 'Set as active'}
						aria-pressed={persona.id === activePersonaId}
						aria-label={persona.id === activePersonaId
							? `${persona.name} is the active persona`
							: `Set ${persona.name} as active`}
						onclick={() => makeActive(persona.id)}
					>
						{persona.id === activePersonaId ? '★' : '☆'}
					</button>
				</div>
			{/each}
		</nav>
	</section>

	<section class="col-content">
		{#if draft}
			<div class="pane">
				<h1>Edit persona</h1>

				<div class="identity-top">
					<div class="identity-fields">
						<div class="field">
							<label for="p-name">Name {@render hint('The persona’s name.')}</label>
							<input id="p-name" bind:value={draft.name} oninput={requestSave} />
						</div>

						<div class="field">
							<label for="p-display">
								Display name
								{@render hint('Shown instead of the name. Leave blank to use the name.')}
							</label>
							<input
								id="p-display"
								placeholder={draft.name}
								bind:value={draft.display_name}
								oninput={requestSave}
							/>
						</div>

						<div class="field">
							<label for="p-pseudonym">
								Pseudonym {@render hint('An alternate name or alias for this persona.')}
							</label>
							<input id="p-pseudonym" bind:value={draft.pseudonym} oninput={requestSave} />
						</div>
					</div>

					<div class="avatar">
						<div class="avatar-preview">
							<TransportImage src={avatarSrc(draft.id)} alt={`${draft.name} avatar`} />
						</div>
						<button type="button" class="pickFile" onclick={openPicker} disabled={busy}>
							{busy ? 'Uploading…' : 'Change avatar'}
						</button>
						{#if avatarError}
							<span class="avatar-error">{avatarError}</span>
						{/if}
					</div>
				</div>

				<div class="field">
					<label for="p-desc">
						Description {@render hint('Free-form notes describing this persona.')}
					</label>
					<textarea id="p-desc" rows="8" bind:value={draft.description} oninput={requestSave}
					></textarea>
				</div>
			</div>
		{:else}
			<div class="pane empty-pane">
				<p class="empty">Select a persona to edit.</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.persona-editor {
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
	.title {
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
	.persona-list {
		display: flex;
		flex-direction: column;
		padding: var(--spacing-sm);
		gap: var(--spacing-xs);
		overflow-y: auto;
	}
	.persona-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-sm);
	}
	.persona-item:hover {
		background-color: var(--color-hover);
	}
	.persona-item.selected {
		background-color: var(--color-selected);
	}
	.persona-item.is-active {
		box-shadow: inset 2px 0 0 var(--color-border-active);
	}
	.persona-open {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		background: transparent;
		padding: var(--spacing-sm);
		text-align: left;
	}
	.active-toggle {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		color: var(--color-text-muted);
		font-size: var(--font-size-md);
		line-height: 1;
	}
	.active-toggle:hover {
		color: var(--color-text-accent);
	}
	.active-toggle.on {
		color: var(--color-text-accent);
	}
	.list-avatar {
		flex: 0 0 auto;
		width: 32px;
		height: 32px;
		overflow: hidden;
		border-radius: var(--radius-full);
		background: var(--gradient-portrait);
	}
	.persona-text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.persona-name {
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.persona-sub {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
	.empty-pane {
		align-items: center;
		justify-content: center;
	}
	.pane h1 {
		font-size: var(--font-size-h3);
		margin: 0;
	}

	/* --- identity row: three single-line fields + avatar --- */
	.identity-top {
		display: flex;
		gap: var(--spacing-lg);
		align-items: stretch;
	}
	.identity-fields {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.field {
		display: flex;
		flex-direction: column;
	}
	.avatar {
		flex: 0 0 120px;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.avatar-preview {
		flex: 1 1 auto;
		width: 120px;
		min-height: 0;
		overflow: hidden;
		border-radius: var(--radius-lg);
		background: var(--gradient-portrait); /* shows while loading or when no avatar is set */
		box-shadow: var(--shadow-md);
	}
	/* TransportImage renders its own wrapper span; make it fill the frame. */
	.avatar-preview :global(.transport-image),
	.list-avatar :global(.transport-image) {
		display: block;
		width: 100%;
		height: 100%;
	}
	.pickFile {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.avatar-error {
		flex: 0 0 auto;
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
		word-break: break-word;
	}

	label {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin-top: var(--spacing-xs);
	}
	.hint {
		flex: 0 0 auto;
		width: 16px;
		height: 16px;
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		line-height: 1;
		cursor: help;
	}
	.hint:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
