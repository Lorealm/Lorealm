<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import DOMPurify from 'dompurify';
	import { marked } from 'marked';
	import type { Snippet } from 'svelte';
	import { ExternalLink } from '@lucide/svelte';

	interface Details {
		name: string;
		version: string;
		author: string;
		description: string;
		tags: string[];
		permissions: string[];
		hosts: string[];
	}

	let {
		plugin,
		readme,
		repository,
		actions,
		notice,
		logs,
		error = ''
	}: {
		plugin: Details;
		/** Fetches the README as Markdown. Fails for a plugin that has none. */
		readme: () => Promise<string>;
		/** Where the plugin is developed, if that's known. */
		repository?: string;
		actions: Snippet;
		notice?: Snippet;
		logs?: Snippet;
		error?: string;
	} = $props();

	/** What each permission lets a plugin do. */
	const permissionLabels: Record<string, string> = $derived(t.plugins.permissions);

	let rendered = $state<string | null | undefined>(undefined);
	let tab = $state<'readme' | 'logs'>('readme');

	$effect(() => {
		rendered = undefined;
		readme()
			.then((markdown) => {
				rendered = DOMPurify.sanitize(marked.parse(callouts(markdown), { async: false }));
			})
			.catch(() => (rendered = null));
	});

	function callouts(markdown: string): string {
		return markdown.replace(
			/^([ \t]*>[ \t]*)\[!(note|tip|important|warning|caution)\][ \t]*$/gim,
			(_, quote: string, kind: string) =>
				`${quote}**${t.plugins.callouts[kind.toLowerCase() as keyof typeof t.plugins.callouts]}**\n${quote.trimEnd()}`
		);
	}

	function openLinksOutside(event: MouseEvent) {
		const link = (event.target as Element).closest('a');
		if (!link?.href) return;
		event.preventDefault();
		window.open(link.href, '_blank', 'noopener');
	}
</script>

<div class="page">
	<aside class="header">
		<h2>{plugin.name} <span class="muted">{plugin.version}</span></h2>
		<span class="muted">{plugin.author}</span>
		{#if plugin.description}<p>{plugin.description}</p>{/if}
		{#if plugin.tags.length}
			<div class="tags">
				{#each plugin.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
			</div>
		{/if}

		<div class="actions">
			{@render actions()}
			{#if repository?.startsWith('https://')}
				<button type="button" onclick={() => window.open(repository, '_blank', 'noopener')}>
					<ExternalLink size={14} />
					{t.plugins.repository}
				</button>
			{/if}
		</div>
		{@render notice?.()}
		{#if error}<p class="error">{error}</p>{/if}

		<h3>{t.plugins.permissionsTitle}</h3>
		{#if plugin.permissions.length || plugin.hosts.length}
			<table>
				<tbody>
					{#each plugin.permissions as permission (permission)}
						<tr>
							<td>{permissionLabels[permission] ?? permission}</td>
							<td class="muted"><code>{permission}</code></td>
						</tr>
					{/each}
					{#each plugin.hosts as host (host)}
						<tr>
							<td>{t.plugins.sendRequestsTo}</td>
							<td class="muted"><code>{host}</code></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{:else}
			<p class="muted">{t.plugins.none}</p>
		{/if}
	</aside>

	<div class="body">
		{#if logs}
			<div class="tabs" role="tablist" aria-label={t.plugins.detailTabs.list}>
				{#each ['readme', 'logs'] as const as key (key)}
					<button
						type="button"
						role="tab"
						aria-selected={tab === key}
						class:active={tab === key}
						onclick={() => (tab = key)}
					>
						{t.plugins.detailTabs[key]}
					</button>
				{/each}
			</div>
		{/if}

		{#if logs && tab === 'logs'}
			<section class="content">{@render logs()}</section>
		{:else}
			<section class="content readme">
				{#if rendered === undefined}
					<p class="muted">{t.plugins.loading}</p>
				{:else if rendered === null}
					<p class="muted">{t.plugins.noReadme}</p>
				{:else}
					<!-- Sanitized with DOMPurify. -->
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					<div onclickcapture={openLinksOutside}>{@html rendered}</div>
				{/if}
			</section>
		{/if}
	</div>
</div>

<style>
	.page {
		display: grid;
		grid-template-columns: minmax(260px, 340px) 1fr;
		flex: 1;
		min-height: 0;
	}

	.header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		border-right: 1px solid var(--color-border-subtle);
		overflow-y: auto;
	}
	.header h2 {
		font-size: var(--font-size-h4);
	}
	.header h3 {
		margin-top: var(--spacing-md);
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}
	.header p {
		margin: 0;
	}
	.muted,
	.header :global(.muted) {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-xs);
	}
	.tag {
		padding: 0 var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		font-size: var(--font-size-xs);
	}

	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.actions :global(button) {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: none;
		border: 1px solid var(--color-border-medium);
	}
	.actions :global(button.on) {
		color: var(--color-success-text);
		border-color: var(--color-success-border);
	}
	.actions :global(button.primary) {
		border-color: transparent;
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
	.actions :global(button.icon) {
		margin-left: auto;
		border: none;
		color: var(--color-text-secondary);
	}
	.actions :global(button.icon:hover) {
		color: var(--color-error-text);
		background: none;
	}
	.error {
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}

	table {
		border-collapse: collapse;
		font-size: var(--font-size-sm);
	}
	td {
		padding: var(--spacing-xs) 0;
		border-bottom: 1px solid var(--color-border-subtle);
		vertical-align: top;
	}
	td + td {
		padding-left: var(--spacing-sm);
		text-align: right;
	}
	code {
		font-family: var(--font-family-mono);
	}

	.body {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
	}
	.tabs {
		display: flex;
		flex: 0 0 auto;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tabs button {
		padding: var(--spacing-sm) var(--spacing-md);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-text-muted);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
	}
	.tabs button:hover {
		color: var(--color-text-secondary);
	}
	.tabs button.active {
		border-bottom-color: var(--color-primary-light);
		color: var(--color-text-primary);
	}
	.content {
		flex: 1 1 auto;
		min-height: 0;
		padding: var(--spacing-lg);
		overflow-y: auto;
	}
	.readme {
		line-height: 1.6;
		overflow-wrap: anywhere;
	}
	.readme :global(:is(h1, h2, h3, h4, h5, h6)) {
		margin: var(--spacing-lg) 0 var(--spacing-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
		letter-spacing: normal;
		text-transform: none;
	}
	.readme :global(h1) {
		font-size: var(--font-size-h2);
	}
	.readme :global(h2) {
		padding-bottom: var(--spacing-xs);
		border-bottom: 1px solid var(--color-border-subtle);
		font-size: var(--font-size-h3);
	}
	.readme :global(h3) {
		font-size: var(--font-size-h4);
	}
	.readme :global(h4) {
		font-size: var(--font-size-h5);
	}
	.readme :global(> div > :first-child) {
		margin-top: 0;
	}
	.readme :global(:is(p, ul, ol, pre, blockquote, table)) {
		margin: 0 0 var(--spacing-md);
	}
	.readme :global(:is(ul, ol)) {
		padding-left: var(--spacing-xl);
	}
	.readme :global(blockquote) {
		padding: var(--spacing-sm) var(--spacing-md);
		border-left: 3px solid var(--color-border-medium);
		border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
		background-color: var(--color-bg-elevated);
		color: var(--color-text-secondary);
	}
	.readme :global(blockquote > :last-child) {
		margin-bottom: 0;
	}
	.readme :global(a) {
		color: var(--color-primary-light);
	}
	.readme :global(img) {
		max-width: 100%;
	}
	.readme :global(hr) {
		margin: var(--spacing-lg) 0;
		border: none;
		border-top: 1px solid var(--color-border-subtle);
	}
	.readme :global(:is(th, td)) {
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		text-align: left;
	}
	.readme :global(code) {
		font-family: var(--font-family-mono);
		font-size: 0.9em;
	}
	.readme :global(pre) {
		overflow-x: auto;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-input);
	}

	@media (max-width: 768px) {
		.page {
			display: flex;
			flex-direction: column;
			overflow-y: auto;
		}
	}
</style>
