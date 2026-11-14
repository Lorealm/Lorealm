<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { MessageSquare, Play, X } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { showToast } from '#lib/stores/errorToast.svelte.js';
	import { showApiError } from '#lib/stores/errorActions.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { ConversationMetadata } from '#lib/types/conversation.js';
	import type { Realm, RealmSummary, Scenario } from '#lib/types/realm.js';

	let {
		realm,
		conversations,
		onclose
	}: {
		realm: RealmSummary;
		conversations: ConversationMetadata[];
		onclose: () => void;
	} = $props();

	const api = useApi();

	let full = $state<Realm | null>(null);
	let busy = $state(false);

	$effect(() => {
		api.realms
			.get(realm.id)
			.then((r) => (full = r))
			.catch(() => (full = null));
	});

	const scenarios = $derived<Scenario[]>(full?.definition.scenarios ?? []);

	async function start(slug: string | null) {
		if (busy) return;
		busy = true;
		try {
			const conversation = await api.chat.startRealm({
				realm_id: realm.id,
				scenario: slug
			});
			await chatStore.loadConversation(conversation.metadata.id);
			navigator.start().write();
			showToast({
				variant: 'info',
				message: t.realms.play.started(realm.name || t.realms.play.realm)
			});
			onclose();
		} catch (e) {
			showApiError(e, { title: t.realms.play.startFailed });
		} finally {
			busy = false;
		}
	}

	async function resume(id: string) {
		if (busy) return;
		busy = true;
		try {
			await chatStore.loadConversation(id);
			navigator.start().write();
			onclose();
		} catch (e) {
			showApiError(e, { title: t.realms.play.openFailed });
		} finally {
			busy = false;
		}
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.currentTarget === e.target && onclose()}>
	<div class="dialog" role="dialog" aria-modal="true" aria-label={t.realms.play.dialog(realm.name)}>
		<header>
			<h2>{realm.name || t.realms.cards.untitled}</h2>
			<button type="button" class="close" aria-label={t.common.close} onclick={onclose}>
				<X size={14} />
			</button>
		</header>

		<div class="body">
			<section>
				<h3>{t.realms.play.newPlaythrough}</h3>
				{#each scenarios as scenario (scenario.slug)}
					<button type="button" class="row" disabled={busy} onclick={() => start(scenario.slug)}>
						<Play size={12} />
						<span class="rowText">
							<span class="rowName">{scenario.name}</span>
							{#if scenario.description}
								<span class="rowHint">{scenario.description}</span>
							{/if}
						</span>
					</button>
				{:else}
					<button type="button" class="row" disabled={busy} onclick={() => start(null)}>
						<Play size={12} />
						<span class="rowText">
							<span class="rowName">{t.realms.play.openStart}</span>
							<span class="rowHint">{t.realms.play.noScenarios}</span>
						</span>
					</button>
				{/each}
			</section>

			<section>
				<h3>{t.realms.play.continue}</h3>
				{#each conversations as conversation (conversation.id)}
					<button type="button" class="row" disabled={busy} onclick={() => resume(conversation.id)}>
						<MessageSquare size={12} />
						<span class="rowText">
							<span class="rowName">{conversation.title || t.realms.play.untitled}</span>
							<span class="rowHint">
								{t.realms.play.messages(conversation.length)} · {new Date(
									conversation.updated_at
								).toLocaleDateString()}
							</span>
						</span>
					</button>
				{:else}
					<p class="none">{t.realms.play.nothing}</p>
				{/each}
			</section>
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: var(--color-bg-overlay);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.dialog {
		display: flex;
		flex-direction: column;
		width: min(480px, 92vw);
		max-height: 80vh;
		background-color: var(--color-bg-modal);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-xl);
	}
	header {
		display: flex;
		align-items: center;
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	h2 {
		flex: 1 1 auto;
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h5);
		font-weight: var(--font-weight-semibold);
	}
	.close {
		display: flex;
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-muted);
	}
	.close:hover {
		color: var(--color-text-accent);
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		padding: var(--spacing-md);
		overflow-y: auto;
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	h3 {
		margin: 0 0 var(--spacing-xs);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		padding: var(--spacing-sm);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		text-align: left;
	}
	.row:hover:not(:disabled) {
		border-color: var(--color-border-active);
		color: var(--color-text-primary);
	}
	.rowText {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.rowName {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
	}
	.rowHint {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.none {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
