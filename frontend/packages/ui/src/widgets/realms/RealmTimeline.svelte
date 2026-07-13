<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { Hammer, Loader } from '@lucide/svelte';

	let scroller = $state<HTMLDivElement>();
	let loadingOlder = $state(false);
	let noMoreOlder = $state(false);

	const events = $derived(realmStore.events);
	const turn = $derived(realmStore.turn);
	const awaiting = $derived(realmStore.awaitingAgent);

	// One committed message = one turn; number the groups in arrival order.
	const turnNumbers = $derived.by(() => {
		const map = new Map<string, number>();
		for (const e of events) {
			if (!map.has(e.message_id)) map.set(e.message_id, map.size + 1);
		}
		return map;
	});

	// Stick to the bottom as events stream in.
	$effect(() => {
		void events.length;
		void turn?.text.length;
		void turn?.reasoning.length;
		const el = scroller;
		if (!el) return;
		requestAnimationFrame(() => el.scrollTo({ top: el.scrollHeight }));
	});

	async function loadOlder() {
		if (loadingOlder || noMoreOlder) return;
		loadingOlder = true;
		try {
			const got = await realmStore.loadOlderEvents();
			if (got === 0) noMoreOlder = true;
		} finally {
			loadingOlder = false;
		}
	}

	function sourceName(source: string | null): string {
		if (!source) return 'World';
		return realmStore.agentById(source)?.name ?? 'Unknown';
	}
</script>

<div class="timeline" bind:this={scroller}>
	{#if events.length > 0 && !noMoreOlder}
		<button type="button" class="older" onclick={loadOlder} disabled={loadingOlder}>
			{loadingOlder ? 'Loading…' : 'Load older events'}
		</button>
	{/if}

	{#each events as event, i (event.id)}
		{#if i === 0 || events[i - 1].message_id !== event.message_id}
			<div class="tickMarker">turn {turnNumbers.get(event.message_id)}</div>
		{/if}
		<div class="event" data-kind={event.kind}>
			{#if event.kind === 'spoke'}
				<span class="speaker">{sourceName(event.source)}</span>
				<p class="speech">{event.content}</p>
			{:else if event.kind === 'acted'}
				<span class="line"><Hammer size={12} /> {event.content}</span>
			{:else}
				<span class="line">{event.content}</span>
			{/if}
			<span class="scope" title="Who can observe this event">{event.scope || 'private'}</span>
		</div>
	{:else}
		{#if !turn && !awaiting}
			<div class="empty">
				A fresh timeline. Press <strong>Run</strong> to let the world take its first turn.
			</div>
		{/if}
	{/each}

	{#if turn}
		<div class="tickMarker live">
			<Loader size={12} class="spin" />
			{turn.name} is taking their turn…
		</div>
		<div class="event live" data-kind="spoke">
			<span class="speaker">{turn.name}</span>
			{#if turn.reasoning}
				<details class="reasoning">
					<summary>Reasoning</summary>
					<p>{turn.reasoning}</p>
				</details>
			{/if}
			{#if turn.text}
				<p class="speech">{turn.text}</p>
			{:else}
				<p class="speech typing">…</p>
			{/if}
			{#each turn.actions as action, i (i)}
				<span class="line" class:error={action.is_error}>
					<Hammer size={12} />
					used {action.name}{action.is_error ? ' (failed)' : ''}
				</span>
			{/each}
		</div>
	{/if}

	{#if awaiting && !turn}
		<div class="tickMarker">
			Waiting on <strong>{awaiting.name}</strong> — it's their turn.
		</div>
	{/if}
</div>

<style>
	.timeline {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
	}

	.older {
		align-self: center;
		font-size: 0.8rem;
		color: var(--color-text-muted);
		background: none;
		border: 1px solid var(--color-border-subtle);
		border-radius: 999px;
		padding: 4px 12px;
	}

	.tickMarker {
		align-self: center;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: 0.75rem;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		margin-top: var(--spacing-sm);
	}

	.event {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-width: 720px;
		width: 100%;
		align-self: center;
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-md, 8px);
		position: relative;
	}

	.event[data-kind='spoke'] {
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
	}

	.event[data-kind='acted'] .line,
	.event[data-kind='system'] .line {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-secondary);
		font-style: italic;
		font-size: 0.9rem;
	}

	.event[data-kind='state_changed'] .line {
		color: var(--color-text-muted);
		font-family: var(--font-mono, monospace);
		font-size: 0.8rem;
	}

	.speaker {
		font-weight: 600;
		font-size: 0.85rem;
	}

	.speech {
		margin: 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.typing {
		color: var(--color-text-muted);
	}

	.line.error {
		color: var(--color-danger, #e5484d);
	}

	.scope {
		position: absolute;
		top: var(--spacing-xs);
		right: var(--spacing-sm);
		font-size: 0.7rem;
		color: var(--color-text-muted);
	}

	.reasoning {
		font-size: 0.85rem;
		color: var(--color-text-muted);
	}

	.reasoning p {
		margin: var(--spacing-xs) 0 0;
		white-space: pre-wrap;
	}

	.event.live {
		border-style: dashed;
	}

	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		text-align: center;
	}

	:global(.spin) {
		animation: spin 1.2s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
