<script lang="ts">
	import { REALM_CONFIG_DEFAULTS } from '$lib/types/realm.js';
	import { realmDraft } from '../draft.svelte.js';

	/** Each override, with what it governs and the holes it can use. */
	const FORMATS = [
		{
			key: 'speech_format',
			label: 'Someone speaking',
			about: 'How another agent’s speech reads in an observer’s feed.',
			holes: '{name}, {content}'
		},
		{
			key: 'action_format',
			label: 'Someone acting',
			about: 'The line an agent’s tool use leaves in the feed.',
			holes: '{name}, {tool}'
		},
		{
			key: 'tool_result_format',
			label: 'A tool’s result',
			about: 'The private note the actor sees after its tool runs.',
			holes: '{tool}, {output}'
		}
	] as const;

	function set(key: string, value: string) {
		if (value.trim()) realmDraft.config[key] = value;
		else delete realmDraft.config[key];
	}
</script>

<div class="pane">
	<header>
		<h2>Formatting</h2>
		<span class="hint">how events read in an agent's feed</span>
	</header>

	<div class="body">
		{#each FORMATS as format (format.key)}
			<label>
				<span class="label">{format.label}</span>
				<input
					class="mono"
					value={realmDraft.config[format.key] ?? ''}
					placeholder={REALM_CONFIG_DEFAULTS[format.key]}
					spellcheck="false"
					oninput={(e) => set(format.key, e.currentTarget.value)}
				/>
				<span class="about">
					{format.about}
					<code>{format.holes}</code>
				</span>
			</label>
		{/each}
	</div>
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
	header {
		display: flex;
		flex: 0 0 auto;
		align-items: baseline;
		gap: var(--spacing-sm);
		height: var(--height-header);
		padding: 0 var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.body {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
		max-width: var(--width-content-max);
		overflow-y: auto;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.mono {
		font-family: var(--font-family-mono);
	}
	.about {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.about code {
		font-family: var(--font-family-mono);
		color: var(--color-text-secondary);
	}
</style>
