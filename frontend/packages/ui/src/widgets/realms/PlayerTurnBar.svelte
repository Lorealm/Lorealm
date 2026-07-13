<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { JsonValue } from '$lib/types/core.js';
	import type { Param } from '$lib/types/tools.js';
	import { Send } from '@lucide/svelte';

	let content = $state('');
	let selectedToolId = $state<string>('');
	let argDrafts = $state<Record<string, string>>({});
	let busy = $state(false);

	const awaiting = $derived(realmStore.awaitingAgent);
	const myTurn = $derived(awaiting?.kind === 'player');
	const actions = $derived(realmStore.playerActions);
	const selectedAction = $derived(actions.find((a) => a.tool_id === selectedToolId) ?? null);

	$effect(() => {
		void realmStore.awaitingAgentId;
		selectedToolId = '';
		argDrafts = {};
		realmStore.loadPlayerActions();
	});

	function parseArg(param: Param, raw: string): JsonValue {
		const ty = param.ty;
		if (ty === 'number') return Number(raw);
		if (ty === 'bool') return raw === 'true';
		if (typeof ty === 'object' && 'array' in ty) {
			return raw
				.split(',')
				.map((s) => s.trim())
				.filter((s) => s !== '')
				.map((s) => (ty.array === 'number' ? Number(s) : ty.array === 'bool' ? s === 'true' : s));
		}
		return raw;
	}

	async function submit() {
		if (busy || !myTurn) return;
		const text = content.trim();

		let action = null;
		if (selectedAction) {
			const args: Record<string, JsonValue> = {};
			for (const param of selectedAction.params) {
				const raw = argDrafts[param.name] ?? '';
				if (raw === '' && !param.required) continue;
				args[param.name] = parseArg(param, raw);
			}
			action = { tool_id: selectedAction.tool_id, args: args as JsonValue };
		}

		if (!text && !action) return;

		busy = true;
		try {
			const ok = await realmStore.submitPlayerTurn(text || null, action);
			if (ok) {
				content = '';
				selectedToolId = '';
				argDrafts = {};
			}
		} finally {
			busy = false;
		}
	}
</script>

{#if myTurn && awaiting}
	<div class="turnBar">
		<div class="turnHeader">
			Your turn as <strong>{awaiting.name}</strong>
			{#if actions.length > 0}
				<select bind:value={selectedToolId} aria-label="Choose an action">
					<option value="">No action (just speak)</option>
					{#each actions as action (action.tool_id)}
						<option value={action.tool_id}>{action.display_name ?? action.name}</option>
					{/each}
				</select>
			{/if}
		</div>

		{#if selectedAction}
			<div class="args">
				{#if selectedAction.description}
					<p class="desc">{selectedAction.description}</p>
				{/if}
				{#each selectedAction.params as param (param.name)}
					<label>
						<span>{param.name}{param.required ? ' *' : ''}</span>
						{#if param.ty === 'bool'}
							<select bind:value={argDrafts[param.name]}>
								<option value="">—</option>
								<option value="true">true</option>
								<option value="false">false</option>
							</select>
						{:else}
							<input
								type={param.ty === 'number' ? 'number' : 'text'}
								bind:value={argDrafts[param.name]}
								placeholder={param.description ?? ''}
							/>
						{/if}
					</label>
				{/each}
			</div>
		{/if}

		<div class="inputRow">
			<textarea
				rows="2"
				placeholder="Say something…"
				bind:value={content}
				onkeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault();
						submit();
					}
				}}
			></textarea>
			<button
				type="button"
				class="primary"
				onclick={submit}
				disabled={busy || (!content.trim() && !selectedAction)}
				title="Submit your turn"
			>
				<Send size={16} />
			</button>
		</div>
	</div>
{/if}

<style>
	.turnBar {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
	}

	.turnHeader {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: 0.9rem;
		color: var(--color-text-secondary);
	}

	.turnHeader select {
		width: auto;
		margin-left: auto;
	}

	.args {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
	}

	.desc {
		width: 100%;
		margin: 0;
		font-size: 0.8rem;
		color: var(--color-text-muted);
	}

	.args label {
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: 0.8rem;
		color: var(--color-text-secondary);
	}

	.args input,
	.args select {
		width: 180px;
	}

	.inputRow {
		display: flex;
		align-items: flex-end;
		gap: var(--spacing-sm);
	}

	.inputRow textarea {
		flex: 1;
		resize: none;
	}

	.inputRow button {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-sm);
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
</style>
