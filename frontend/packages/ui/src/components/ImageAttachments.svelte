<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { blobUrl } from '#lib/api/blobs.js';
	import TransportImage from './TransportImage.svelte';
	import { X } from '@lucide/svelte';

	let {
		uris,
		onremove
	}: {
		/** `sha256:` URIs. */
		uris: string[];
		/** Shows a remove button on each image. */
		onremove?: (uri: string) => void;
	} = $props();

	let dialog: HTMLDialogElement;
	let shown = $state<string | null>(null);

	function open(uri: string) {
		shown = uri;
		dialog.showModal();
	}

	function onclick(event: MouseEvent) {
		if ('closedBy' in HTMLDialogElement.prototype || event.target !== dialog) return;
		const r = dialog.getBoundingClientRect();
		const inside =
			r.top <= event.clientY &&
			event.clientY <= r.bottom &&
			r.left <= event.clientX &&
			event.clientX <= r.right;
		if (!inside) dialog.close();
	}
</script>

<div class="images">
	{#each uris as uri (uri)}
		<span class="thumb">
			<button
				type="button"
				class="open"
				aria-label={t.chat.message.image}
				onclick={() => open(uri)}
			>
				<TransportImage src={blobUrl(uri)} alt={t.chat.message.image} />
			</button>
			{#if onremove}
				<button
					type="button"
					class="remove"
					aria-label={t.chat.composer.removeImage}
					onclick={() => onremove(uri)}
				>
					<X size={11} />
				</button>
			{/if}
		</span>
	{/each}
</div>

<dialog
	bind:this={dialog}
	{...{ closedby: 'any' } as Record<string, string>}
	aria-label={t.chat.message.image}
	{onclick}
	onclose={() => (shown = null)}
>
	{#if shown}
		<TransportImage src={blobUrl(shown)} alt={t.chat.message.image} />
	{/if}
	<form method="dialog">
		<button>{t.chat.message.closeImage}</button>
	</form>
</dialog>

<style>
	.images {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
	}
	.thumb {
		position: relative;
		display: inline-flex;
	}
	.open {
		display: block;
		width: 8rem;
		height: 8rem;
		padding: 0;
		overflow: hidden;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background: var(--color-bg-base);
		cursor: zoom-in;
	}
	.open :global(.transport-image),
	.open :global(img) {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.remove {
		position: absolute;
		top: 2px;
		right: 2px;
		display: flex;
		padding: 2px;
		border: none;
		border-radius: var(--radius-full);
		background: var(--color-bg-elevated);
		color: var(--color-text-secondary);
	}
	.remove:hover {
		color: var(--color-text-accent);
	}

	dialog {
		max-width: 90vw;
		max-height: 90vh;
		padding: var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		background: var(--color-bg-elevated);
		color: var(--color-text-primary);
	}
	dialog:modal {
		overscroll-behavior: contain;
	}
	dialog::backdrop {
		overflow: hidden;
		overscroll-behavior: contain;
		background-color: rgb(0 0 0 / 0.7);
	}
	dialog :global(img) {
		display: block;
		max-width: calc(90vw - 2 * var(--spacing-sm));
		max-height: calc(90vh - 4rem);
		object-fit: contain;
	}
	dialog form {
		display: flex;
		justify-content: flex-end;
		margin-top: var(--spacing-sm);
	}
</style>
