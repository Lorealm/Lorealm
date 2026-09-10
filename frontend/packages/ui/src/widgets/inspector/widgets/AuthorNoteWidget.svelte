<script lang="ts">
	import ContextField from '$lib/components/ContextField.svelte';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { ChevronDown } from '@lucide/svelte';
	import { onDestroy, untrack } from 'svelte';

	const DEBOUNCE = 500;

	let collapsed = $state(false);
	let timer: ReturnType<typeof setTimeout> | null = null;
	/** The edit waiting on the debounce */
	let queued: { id: Snowflake; note: string } | null = null;

	let conversation = $derived(chatStore.conversation);
	/** A greeting is on screen but `start` hasn't created the row yet. */
	let seeding = $derived(!conversation && chatStore.isPending);
	// Seeded at init
	let draft = $state(chatStore.conversation?.author_note ?? chatStore.pendingAuthorNote);
	let lastId: Snowflake | null = chatStore.conversation?.id ?? null;

	$effect(() => {
		const id = conversation?.id ?? null;
		if (id === lastId) return;
		lastId = id;
		untrack(() => {
			flush();
			draft = conversation?.author_note ?? chatStore.pendingAuthorNote;
		});
	});

	onDestroy(flush);

	function flush() {
		if (timer) clearTimeout(timer);
		timer = null;
		if (!queued) return;
		const { id, note } = queued;
		queued = null;
		void chatStore.setAuthorNote(id, note);
	}

	function onchange(next: string) {
		draft = next;

		// Nothing to PATCH before the row exists — the store carries the seed
		// until `start` sends it.
		if (seeding) {
			chatStore.setPendingAuthorNote(next);
			return;
		}

		const id = conversation?.id;
		if (id === undefined) return;
		queued = { id, note: next };
		if (timer) clearTimeout(timer);
		timer = setTimeout(flush, DEBOUNCE);
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">Author's Note</span>
	</button>

	{#if !collapsed}
		{#if !conversation && !seeding}
			<div class="empty">No active chat.</div>
		{:else}
			<div class="body">
				<ContextField
					value={draft}
					{onchange}
					placeholder="&lbrace;&lbrace;char:name&rbrace;&rbrace; has achieved something remarkable..."
					surface="sunken"
					fill
				/>
			</div>
		{/if}
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-height, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		text-align: left;
		cursor: pointer;
	}
	.label {
		flex: 1;
		min-width: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev :global(svg) {
		width: 14px;
		height: 14px;
	}

	.body {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		min-width: 0;
		padding: var(--spacing-sm) var(--spacing-md);
	}
	.empty {
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
