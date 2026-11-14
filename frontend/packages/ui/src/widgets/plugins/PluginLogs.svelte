<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { RotateCw } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import type { PluginLog, PluginLogLevel } from '#lib/types/plugins.js';
	import { SvelteSet } from 'svelte/reactivity';

	let { handle }: { handle: string } = $props();

	const api = useApi();

	const levels: PluginLogLevel[] = ['trace', 'debug', 'info', 'warn', 'error'];
	const periods = {
		'15m': 15 * 60_000,
		'1h': 3_600_000,
		'24h': 86_400_000,
		'7d': 604_800_000,
		all: Infinity
	};
	type Period = keyof typeof periods;

	let logs = $state<PluginLog[] | null>(null);
	let failed = $state(false);
	let loading = $state(false);
	let shown = $state<Set<PluginLogLevel>>(new Set(['info', 'warn', 'error']));
	let period = $state<Period>('24h');

	const visible = $derived.by(() => {
		const since = Date.now() - periods[period];
		return (logs ?? []).filter(
			(log) => shown.has(log.level) && Date.parse(log.created_at) >= since
		);
	});

	async function load() {
		loading = true;
		try {
			logs = await api.plugins.logs(handle);
			failed = false;
		} catch {
			failed = true;
		}
		loading = false;
	}

	$effect(() => {
		void handle;
		logs = null;
		void load();
	});

	function toggle(level: PluginLogLevel) {
		const next = new SvelteSet(shown);
		if (next.has(level)) next.delete(level);
		else next.add(level);
		shown = next;
	}

	const time = (iso: string) =>
		new Date(iso).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'medium' });
</script>

<div class="logs">
	<div class="filters">
		<div class="levels" role="group" aria-label={t.plugins.logs.levelFilter}>
			{#each levels as level (level)}
				<button
					type="button"
					class="level {level}"
					aria-pressed={shown.has(level)}
					onclick={() => toggle(level)}
				>
					{t.plugins.logs.levels[level]}
				</button>
			{/each}
		</div>
		<select bind:value={period} aria-label={t.plugins.logs.period}>
			{#each Object.keys(periods) as key (key)}
				<option value={key}>{t.plugins.logs.periods[key as Period]}</option>
			{/each}
		</select>
		<button
			type="button"
			class="refresh"
			aria-label={t.plugins.logs.refresh}
			{@attach tooltip(t.plugins.logs.refresh)}
			disabled={loading}
			onclick={load}
		>
			<RotateCw size={14} />
		</button>
	</div>

	{#if failed}
		<p class="note error">{t.plugins.logs.failed}</p>
	{:else if logs === null}
		<p class="note">{t.plugins.loading}</p>
	{:else if !logs.length}
		<p class="note">{t.plugins.logs.empty}</p>
	{:else if !visible.length}
		<p class="note">{t.plugins.logs.noMatch}</p>
	{:else}
		<ol>
			{#each visible as log (log.id)}
				<li>
					<time datetime={log.created_at}>{time(log.created_at)}</time>
					<span class="tag {log.level}">{t.plugins.logs.levels[log.level]}</span>
					<span class="message">
						{#if log.source === 'host'}<span class="host">{t.plugins.logs.fromServer}:</span>{/if}
						<span>{log.message}</span>
					</span>
				</li>
			{/each}
		</ol>
	{/if}
</div>

<style>
	.logs {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.filters {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		overflow-x: auto;
	}
	.filters > :global(*) {
		flex-shrink: 0;
	}
	.filters select {
		width: auto;
	}
	.levels {
		display: flex;
		gap: var(--spacing-xs);
		margin-right: auto;
	}
	.level {
		padding: 0 var(--spacing-sm);
		background: none;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.level[aria-pressed='true'] {
		border-color: currentColor;
		color: var(--level);
	}
	.refresh {
		display: flex;
		align-items: center;
		background: none;
		border: 1px solid var(--color-border-medium);
	}

	.trace {
		--level: var(--color-text-muted);
	}
	.debug {
		--level: var(--color-text-secondary);
	}
	.info {
		--level: var(--color-info-text);
	}
	.warn {
		--level: var(--color-warning-text);
	}
	.error {
		--level: var(--color-error-text);
	}

	.note {
		margin: 0;
		color: var(--color-text-muted);
	}
	.note.error {
		color: var(--color-error-text);
	}

	ol {
		margin: 0;
		padding: 0;
		list-style: none;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
	}
	li {
		display: grid;
		grid-template-columns: auto 5ch 1fr;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	time {
		color: var(--color-text-muted);
		white-space: nowrap;
	}
	.tag {
		color: var(--level);
		font-weight: var(--font-weight-semibold);
	}
	.message {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.host {
		color: var(--color-text-muted);
	}
</style>
