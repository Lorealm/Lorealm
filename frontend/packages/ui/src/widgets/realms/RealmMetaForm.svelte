<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';

	const realm = $derived(realmStore.realm);

	let name = $state(realmStore.realm?.definition.name ?? '');
	let description = $state(realmStore.realm?.definition.description ?? '');
	let tagsDraft = $state((realmStore.realm?.tags ?? []).join(', '));
	let busy = $state(false);

	const dirty = $derived(
		realm !== null &&
			(name !== realm.definition.name ||
				description !== realm.definition.description ||
				tagsDraft !== realm.tags.join(', '))
	);

	async function save() {
		if (!realm || busy || !name.trim()) return;
		busy = true;
		try {
			const tags = tagsDraft
				.split(',')
				.map((t) => t.trim())
				.filter((t) => t !== '');
			const ok = await realmStore.updateRealm({ name: name.trim(), description, tags });
			if (ok) {
				name = realmStore.realm?.definition.name ?? name;
				tagsDraft = (realmStore.realm?.tags ?? []).join(', ');
			}
		} finally {
			busy = false;
		}
	}
</script>

<form
	class="meta"
	onsubmit={(e) => {
		e.preventDefault();
		save();
	}}
>
	<label>
		<span>Name</span>
		<input type="text" bind:value={name} required />
	</label>
	<label>
		<span>Description</span>
		<textarea rows="3" bind:value={description} placeholder="What is this realm?"></textarea>
	</label>
	<label>
		<span>Tags</span>
		<input type="text" bind:value={tagsDraft} placeholder="comma, separated, tags" />
	</label>
	<div class="actions">
		<button type="submit" class="primary" disabled={busy || !dirty || !name.trim()}>
			{busy ? 'Saving…' : 'Save'}
		</button>
	</div>
</form>

<style>
	.meta {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	label > span {
		font-size: 0.8rem;
		color: var(--color-text-secondary);
	}

	textarea {
		resize: vertical;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}

	button.primary:disabled {
		opacity: 0.5;
	}
</style>
