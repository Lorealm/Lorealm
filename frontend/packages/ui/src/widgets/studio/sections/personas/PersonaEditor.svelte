<script lang="ts">
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onDestroy } from 'svelte';
	import { navigator } from '$lib/navigation/navigator.js';
	import { personaStore } from '$lib/stores/persona.svelte.js';
	import { exportItem } from '$lib/stores/transfer.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { blobUrl } from '$lib/api/blobs.js';
	import ContextField from '$lib/components/ContextField.svelte';
	import ListSidebar from '$lib/components/ListSidebar.svelte';
	import { Star, Trash2, X } from '@lucide/svelte';
	import { showApiError } from '$lib/stores/errorActions.js';

	const SAVE_MS = 500;

	let { selectedId }: { selectedId: Snowflake | null } = $props();

	const draft = $derived(personaStore.editingPersona);
	const save = $derived(personaStore.saveState);
	const personas = $derived(personaStore.loadedPersonas ?? []);
	const activePersonaId = $derived(personaStore.activePersonaId);
	const characters = $derived(characterStore.metadataList);

	let query = $state('');
	let sort = $state('name-asc');

	const shownPersonas = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const out = personas.filter(
			(p) =>
				needle === '' ||
				p.name.toLowerCase().includes(needle) ||
				(p.display_name ?? '').toLowerCase().includes(needle)
		);
		if (sort === 'name-desc') return out.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'active')
			return out.sort(
				(a, b) => Number(b.id === activePersonaId) - Number(a.id === activePersonaId)
			);
		return out.sort((a, b) => a.name.localeCompare(b.name));
	});

	function avatarSrc(id: Snowflake): string | null {
		return blobUrl(personas.find((p) => p.id === id)?.avatar_uri);
	}

	let inputEl: HTMLInputElement;
	let busy = $state(false);
	let avatarError = $state<string | null>(null);

	let creating = $state(false);
	async function createPersona() {
		if (creating) return;
		creating = true;
		try {
			const persona = await personaStore.create();
			await select(persona.id);
		} catch (e) {
			console.error(e);
		} finally {
			creating = false;
		}
	}

	const boundCharacters = $derived.by(() => {
		const id = personaStore.editingId;
		if (!id) return [];
		const bound = new Set(personaStore.overridesFor(id).map((o) => o.character_id));
		return characters.filter((c) => bound.has(c.id));
	});

	const bindable = $derived.by(() => {
		const id = personaStore.editingId;
		return characters
			.filter((c) => personaStore.overrideFor(c.id)?.persona_id !== id)
			.map((c) => {
				const other = personaStore.overrideFor(c.id);
				const owner = other && personas.find((p) => p.id === other.persona_id);
				return { id: c.id, label: owner ? `${c.name} — bound to ${owner.name}` : c.name };
			});
	});

	let bindError = $state<string | null>(null);

	async function bind(e: Event) {
		const el = e.currentTarget as HTMLSelectElement;
		const characterId = el.value;
		el.value = '';
		const personaId = personaStore.editingId;
		if (!characterId || !personaId) return;

		bindError = null;
		try {
			await personaStore.setOverride(characterId, personaId);
		} catch (err) {
			bindError = err instanceof Error ? err.message : 'Could not bind that character';
		}
	}

	async function unbind(characterId: Snowflake) {
		bindError = null;
		try {
			await personaStore.clearOverride(characterId);
		} catch (err) {
			bindError = err instanceof Error ? err.message : 'Could not unbind that character';
		}
	}

	let confirmingDelete = $state(false);
	let deleting = $state(false);
	const isLastPersona = $derived(personas.length <= 1);

	async function deletePersona() {
		const id = personaStore.editingId;
		if (!id) return;
		clearTimeout(timer);
		deleting = true;
		try {
			await personaStore.delete(id);
			navigator.start().studio().personas();
		} catch (e) {
			showApiError(e, { title: 'Could not delete persona' });
		} finally {
			deleting = false;
			confirmingDelete = false;
		}
	}

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void personaStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function select(id: Snowflake) {
		if (id === selectedId) return;
		clearTimeout(timer);
		await personaStore.save(); // flush any pending edits before switching (no-op if none)
		navigator.start().studio().personas(id);
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
	<button type="button" class="hint" {@attach tooltip(text)} aria-label={`Help: ${text}`}>?</button>
{/snippet}

<div class="persona-editor">
	<ListSidebar
		title="Personas"
		detailOpen={selectedId !== null}
		oncreate={createPersona}
		{creating}
		bind:query
		bind:sort
		sortOptions={[
			{ value: 'name-asc', label: 'Name (A–Z)' },
			{ value: 'name-desc', label: 'Name (Z–A)' },
			{ value: 'active', label: 'Active first' }
		]}
		searchPlaceholder="Search personas…"
	>
		{#snippet status()}
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span>{saveLabel}</span>
				</div>
			{/if}
			{#if save.status === 'error' && save.message}
				<pre class="save-error">{save.message}</pre>
			{/if}
		{/snippet}

		{#if shownPersonas.length === 0}
			<p class="empty">
				{personas.length === 0 ? 'No personas yet.' : 'No personas match that search.'}
			</p>
		{/if}
		{#each shownPersonas as persona (persona.id)}
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
					class="icon-button active-toggle"
					class:on={persona.id === activePersonaId}
					type="button"
					{@attach tooltip(persona.id === activePersonaId ? 'Active persona' : 'Set as active')}
					aria-pressed={persona.id === activePersonaId}
					aria-label={persona.id === activePersonaId
						? `${persona.name} is the active persona`
						: `Set ${persona.name} as active`}
					onclick={() => makeActive(persona.id)}
				>
					<Star size={14} fill={persona.id === activePersonaId ? 'currentColor' : 'none'} />
				</button>
			</div>
		{/each}
	</ListSidebar>

	<section class="col-content">
		{#if draft}
			<div class="pane">
				<div class="pane-head">
					<h1>Edit persona</h1>
					<button type="button" onclick={() => exportItem('persona', draft.id, draft.name)}>
						Export
					</button>
				</div>

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

				{#key draft}
					<ContextField
						label="Description"
						hint="Free-form notes describing this persona. Reaches the model as {'{{persona:description}}'}."
						value={draft.description}
						rows={8}
						onchange={(v) => {
							draft.description = v;
							requestSave();
						}}
					/>
				{/key}

				<section class="overrides">
					<div class="overrides-head">
						<h2>Character overrides</h2>
						<select onchange={bind} aria-label="Bind a character to this persona">
							<option value="">Bind a character…</option>
							{#each bindable as option (option.id)}
								<option value={option.id}>{option.label}</option>
							{/each}
						</select>
					</div>
					<p class="overrides-note">
						Chats with these characters use this persona instead of your active one. A character can
						only be bound to one persona.
					</p>

					{#if bindError}
						<p class="overrides-error">{bindError}</p>
					{/if}

					{#if boundCharacters.length === 0}
						<p class="empty">No characters are bound to this persona.</p>
					{:else}
						<ul class="bound-list">
							{#each boundCharacters as character (character.id)}
								<li class="bound-item">
									<span class="bound-avatar">
										<TransportImage src={blobUrl(character.avatar_uri)} alt="" />
									</span>
									<span class="bound-name">{character.name}</span>
									<button
										type="button"
										class="unbind"
										{@attach tooltip('Remove override')}
										aria-label={`Unbind ${character.name}`}
										onclick={() => unbind(character.id)}
									>
										<X size={14} />
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</section>

				<section class="danger-zone">
					{#if confirmingDelete}
						<p class="danger-text">
							Delete {draft.name || 'this persona'}? Characters bound to it go back to using your
							active persona.
						</p>
						<div class="danger-buttons">
							<button type="button" class="danger" disabled={deleting} onclick={deletePersona}>
								Delete
							</button>
							<button type="button" disabled={deleting} onclick={() => (confirmingDelete = false)}>
								Cancel
							</button>
						</div>
					{:else}
						<button
							type="button"
							class="delete-persona"
							disabled={isLastPersona}
							onclick={() => (confirmingDelete = true)}
						>
							<Trash2 size={14} />
							Delete persona
						</button>
						{#if isLastPersona}
							<p class="danger-text">
								You need at least one persona, so this one can't be deleted.
							</p>
						{/if}
					{/if}
				</section>
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
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
		overflow-y: auto;
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
	.icon-button {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-muted);
	}

	.icon-button:hover {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-accent);
	}
	.icon-button:focus-visible {
		outline: none;
		box-shadow: 0 0 0 2px var(--color-focus);
	}
	.active-toggle {
		margin-right: var(--spacing-xs);
	}
	.active-toggle.on {
		color: var(--color-primary);
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
	.pane-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
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

	/* --- character overrides --- */
	.overrides {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		margin-top: var(--spacing-lg);
		padding-top: var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
	}
	.overrides-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
	}
	.overrides h2 {
		margin: 0;
		font-size: var(--font-size-h4);
	}
	.overrides-head select {
		width: auto;
		max-width: 260px;
		font-size: var(--font-size-sm);
	}
	.overrides-note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.overrides-error {
		margin: 0;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
	.bound-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.bound-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated-2);
	}
	.bound-avatar {
		flex: 0 0 auto;
		width: 28px;
		height: 28px;
		overflow: hidden;
		border-radius: var(--radius-full);
		background: var(--gradient-portrait);
	}
	.bound-avatar :global(.transport-image) {
		display: block;
		width: 100%;
		height: 100%;
	}
	.bound-name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.unbind {
		flex: 0 0 auto;
		padding: var(--spacing-xs);
		background-color: transparent;
		color: var(--color-text-muted);
	}
	.unbind:hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}

	.danger-zone {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--spacing-sm);
		margin-top: var(--spacing-lg);
		padding-top: var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
	}
	.danger-text {
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.danger-buttons {
		display: flex;
		gap: var(--spacing-xs);
	}
	.delete-persona {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.delete-persona:not(:disabled):hover,
	.danger {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.danger:not(:disabled):hover {
		background-color: var(--color-error);
		color: var(--color-text-white);
	}
	.delete-persona:disabled {
		opacity: 0.5;
		cursor: default;
	}

	@media (max-width: 768px) {
		.pane {
			padding: var(--spacing-md);
		}
	}
</style>
