<script lang="ts">
	import { Download, X } from '@lucide/svelte';
	import { blueprintStore } from '$lib/stores/blueprint.svelte.js';
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import type { BundlePreview } from '$lib/types/blueprint.js';
	import { extractError } from '$lib/types/error.js';
	import type { RealmResourceKind } from '$lib/types/realm.js';
	import Badge from './Badge.svelte';
	import { reload, RESOURCE_KINDS } from './resources.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';

	let { file, onclose }: { file: File; onclose: () => void } = $props();

	const LABELS: Record<RealmResourceKind, string> = {
		character: 'Character',
		lorebook: 'Lorebook',
		template: 'Template',
		tool: 'Tool',
		persona: 'Persona',
		connection_profile: 'Connection profile',
		widget: 'Widget',
		module: 'Module'
	};

	let preview = $state<BundlePreview | null>(null);
	let refusal = $state<string | null>(null);
	let importing = $state(false);

	$effect(() => {
		preview = null;
		refusal = null;
		blueprintStore
			.previewImport(file)
			.then((p) => (preview = p))
			.catch((e) => (refusal = extractError(e).message));
	});

	const blocked = $derived(preview?.resources.some((r) => r.status === 'blocked') ?? false);
	const added = $derived(preview?.resources.filter((r) => r.status === 'new').length ?? 0);

	function cancel() {
		if (!importing) onclose();
	}

	async function run() {
		importing = true;
		const imported = await blueprintStore.import(file);
		importing = false;
		if (!imported) return;

		reload(RESOURCE_KINDS);
		const notes = [
			...imported.renamed.map(([from, to]) => `${from} was renamed to ${to}.`),
			...(imported.skipped.length > 0
				? [`${imported.skipped.join(', ')} couldn't be read and was left out.`]
				: [])
		];
		showToast({
			title: 'Blueprint imported',
			message: [`${imported.blueprint.name} is in your blueprints.`, ...notes].join(' '),
			variant: 'success',
			actions: [
				{
					label: 'Take me there!',
					run: () => {
						navigator.start().blueprints();
					}
				}
			]
		});
		onclose();
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && cancel()} />

<div class="overlay" role="presentation" onclick={(e) => e.currentTarget === e.target && cancel()}>
	<div class="dialog" role="dialog" aria-modal="true" aria-label="Import blueprint">
		<header>
			<h2>Import blueprint</h2>
			<button type="button" class="close" aria-label="Cancel" onclick={cancel}>
				<X size={14} />
			</button>
		</header>

		<div class="body">
			{#if refusal}
				<p class="refusal">{refusal}</p>
			{:else if !preview}
				<p class="quiet">Reading {file.name}…</p>
			{:else}
				<div class="about">
					<span class="title">{preview.name}</span>
					{#if preview.description}<p class="quiet">{preview.description}</p>{/if}
				</div>

				{#if preview.resources.length === 0}
					<p class="quiet">
						This blueprint comes on its own. Nothing else is added to your library.
					</p>
				{:else}
					<p class="lead">
						{#if added === 0}
							Everything this blueprint uses is already in your library.
						{:else}
							Importing adds {added === 1 ? 'this' : `these ${added}`} to your library:
						{/if}
					</p>
					<ul class="resources">
						{#each preview.resources as resource, i (i)}
							<li class:blocked={resource.status === 'blocked'}>
								<div class="row">
									<Badge tone={resource.kind === 'module' ? 'warning' : 'neutral'}>
										{LABELS[resource.kind]}
									</Badge>
									<span class="name">{resource.name}</span>
									{#if resource.status === 'existing'}
										<span class="note">Already in your library</span>
									{/if}
								</div>
								{#if resource.includes.length > 0}
									<span class="includes">
										Comes with {resource.includes
											.map((inc) => `${LABELS[inc.kind].toLowerCase()} ${inc.name}`)
											.join(', ')}
									</span>
								{/if}
								{#if resource.problem}<span class="problem">{resource.problem}</span>{/if}
							</li>
						{/each}
					</ul>
					{#if preview.resources.some((r) => r.kind === 'module' && r.status === 'new')}
						<p class="quiet">Modules are code that runs inside your realms.</p>
					{/if}
				{/if}
			{/if}
		</div>

		<footer>
			<span class="spacer"></span>
			<button type="button" onclick={cancel} disabled={importing}>
				{refusal ? 'Close' : 'Cancel'}
			</button>
			{#if !refusal}
				<button
					type="button"
					class="primary"
					disabled={!preview || blocked || importing}
					onclick={run}
				>
					<Download size={12} />
					{importing ? 'Importing…' : 'Import'}
				</button>
			{/if}
		</footer>
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
		width: min(520px, 92vw);
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
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		overflow-y: auto;
	}
	.about {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.title {
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-semibold);
		color: var(--color-text-primary);
	}
	p {
		margin: 0;
		font-size: var(--font-size-sm);
	}
	.lead {
		color: var(--color-text-secondary);
	}
	.quiet {
		color: var(--color-text-muted);
	}
	.refusal {
		color: var(--color-error-text);
	}
	.resources {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	li.blocked {
		border-color: var(--color-error-text);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
	}
	.note,
	.includes {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.note {
		flex: 0 0 auto;
	}
	.problem {
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
	}
	footer {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-top: 1px solid var(--color-border-subtle);
	}
	.spacer {
		flex: 1 1 auto;
	}
	footer button {
		padding: var(--spacing-xs) var(--spacing-md);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	footer .primary {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
	footer .primary:disabled {
		opacity: 0.5;
	}
</style>
