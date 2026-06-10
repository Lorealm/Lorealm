<script lang="ts">
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { navigationStore, SidebarView } from '$lib/stores/navigation.svelte.js';
	import { StudioSection, studioStore } from '$lib/stores/studio.svelte.js';
	import type { CharacterSummary } from '$lib/types/character.js';
	import { MessageCircle, Pencil } from '@lucide/svelte';

	let { summary }: { summary: CharacterSummary } = $props();
	let imageEndpoint = $derived(`/api/v1/characters/${summary.id}/image`);

	async function startChat() {
		characterStore.setActive(summary.id);
		navigationStore.setSidebarView(SidebarView.Write);

		const opened = await chatStore.openLatestForCharacter(summary.id);
		if (!opened) {
			const character = characterStore.activeCharacter;
			if (character) {
				chatStore.beginNewConversation({
					characterId: character.id,
					versionId: character.current_version_id,
					greetings: character.definition.greetings
				});
			}
		}
	}
</script>

<div class="card">
	<TransportImage src={imageEndpoint} alt={summary.name} />

	<div class="overlay">
		<h3 class="name">{summary.display_name ?? summary.name}</h3>

		<div class="actions">
			<button class="btn btn-chat" type="button" onclick={startChat}>
				<MessageCircle size={16} aria-hidden="true" />
				<span>Chat</span>
			</button>
			<button
				class="btn btn-edit"
				type="button"
				aria-label="Edit"
				onclick={() => {
					studioStore.setSection(StudioSection.Characters);
					studioStore.open(summary.id);
					navigationStore.setSidebarView(SidebarView.Studio);
				}}
			>
				<Pencil size={16} aria-hidden="true" />
			</button>
		</div>
	</div>
</div>

<style>
	.card {
		position: relative;
		width: 100%;
		aspect-ratio: 3 / 4;
		border-radius: var(--radius-xl);
		overflow: hidden;
		box-shadow: var(--shadow-md);
		transition: box-shadow var(--transition-fast) var(--easing-default);
	}

	.card:hover {
		box-shadow: var(--shadow-lg);
	}

	.card :global(img) {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.card::before {
		content: '';
		position: absolute;
		inset: 0;
		background: var(--color-overlay-light);
		opacity: 0;
		transition: opacity var(--transition-fast) var(--easing-default);
		pointer-events: none;
		z-index: var(--z-index-base);
	}

	.card:hover::before,
	.card:focus-within::before {
		opacity: 1;
	}

	.overlay {
		position: absolute;
		inset: auto 0 0 0;
		padding: var(--spacing-lg);
		color: var(--color-text-white);
		z-index: var(--z-index-elevated);

		background: var(--color-overlay-bg);
		backdrop-filter: var(--backdrop-blur-sm);
		-webkit-backdrop-filter: var(--backdrop-blur-sm);

		-webkit-mask-image: linear-gradient(to top, #000 97%, transparent);
		mask-image: linear-gradient(to top, #000 97%, transparent);
	}
	.name {
		margin: 0;
		font-family: var(--font-family-heading);
		font-weight: var(--font-weight-heading-quicksand);
		font-size: var(--font-size-h4);
		line-height: var(--line-height-tight);
	}

	.actions {
		display: flex;
		gap: var(--spacing-sm);
		margin-top: var(--spacing-sm);
		opacity: 0;
		transform: translateY(var(--spacing-xs));
		pointer-events: none;
		transition:
			opacity var(--transition-fast) var(--easing-default),
			transform var(--transition-fast) var(--easing-default);
	}

	.card:hover .actions,
	.card:focus-within .actions {
		opacity: 1;
		transform: translateY(0);
		pointer-events: auto;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-medium);
		cursor: pointer;
		transition:
			background-color var(--transition-fast) var(--easing-default),
			border-color var(--transition-fast) var(--easing-default);
	}

	.btn-chat {
		padding: var(--spacing-xs) var(--spacing-md);
		background: var(--color-primary);
		color: var(--color-text-white);
	}

	.btn-chat:hover {
		background: var(--color-primary-dark);
	}

	.btn-edit {
		padding: var(--spacing-xs);
		background: var(--color-bg-overlay);
		border-color: var(--color-border-medium);
		color: var(--color-text-primary);
	}

	.btn-edit:hover {
		background: var(--color-hover);
		border-color: var(--color-border-active);
	}
</style>
