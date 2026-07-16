<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import Tag from './Tag.svelte';

	const def = realmStore.realm!.definition;

	let name = $state(def.name);
	let description = $state(def.description);
	let tagDraft = $state('');

	const tags = $derived(realmStore.realm?.tags ?? []);

	let timer: ReturnType<typeof setTimeout> | undefined;
	function saveSoon() {
		clearTimeout(timer);
		timer = setTimeout(() => {
			void realmStore.updateRealm({ name: name.trim(), description });
		}, 500);
	}

	function addTag() {
		const t = tagDraft.trim();
		if (!t || tags.includes(t)) {
			tagDraft = '';
			return;
		}
		tagDraft = '';
		void realmStore.updateRealm({ tags: [...tags, t] });
	}

	function removeTag(tag: string) {
		void realmStore.updateRealm({ tags: tags.filter((t) => t !== tag) });
	}
</script>

<div class="stage">
	<div class="field">
		<div class="label">Name</div>
		<input
			class="title"
			type="text"
			bind:value={name}
			placeholder="Untitled Realm"
			oninput={saveSoon}
		/>
	</div>

	<div class="field">
		<div class="label">Description</div>
		<textarea
			rows="4"
			bind:value={description}
			placeholder="What's this world about?"
			oninput={saveSoon}
		></textarea>
	</div>

	<div class="field">
		<div class="label">Tags</div>
		{#if tags.length > 0}
			<div class="tags">
				{#each tags as tag (tag)}
					<Tag onclick={() => removeTag(tag)}>{tag} ✕</Tag>
				{/each}
			</div>
		{/if}
		<div class="addTag">
			<input
				type="text"
				bind:value={tagDraft}
				placeholder="Add a tag"
				onkeydown={(e) => {
					if (e.key === 'Enter') addTag();
				}}
			/>
			<button type="button" class="secondary" onclick={addTag}>Add</button>
		</div>
	</div>
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-2xl);
		max-width: var(--width-content-max);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.label {
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.title {
		font-size: var(--font-size-md);
		font-family: var(--font-family-heading);
		font-weight: 600;
	}

	textarea {
		width: 100%;
		resize: vertical;
		font-family: var(--font-family-body);
		line-height: var(--line-height-normal);
	}

	.tags {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.addTag {
		display: flex;
		gap: var(--spacing-sm);
	}

	.addTag input {
		width: 200px;
	}
</style>
