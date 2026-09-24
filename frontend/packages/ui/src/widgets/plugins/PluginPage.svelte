<script lang="ts">
	import DOMPurify from 'dompurify';
	import { marked } from 'marked';
	import { onMount } from 'svelte';
	import { Check, Trash2 } from '@lucide/svelte';
	import { useApi } from '$lib/api/client.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import type { InputSpec } from '$lib/types/blueprint.js';
	import InputControl, { blankFor } from '../realms/InputControl.svelte';

	let { handle }: { handle: string } = $props();

	const api = useApi();

	/** What each permission lets a plugin do. */
	const permissionLabels: Record<string, string> = {
		'data:persist': 'Store its own data',
		'persona:read': 'Read your active persona',
		'prompt:write': 'Change prompts before they are sent',
		'state:read': 'Read script variables',
		'state:write': 'Change script variables',
		'ui:unsafe': 'Run code directly in the app'
	};

	const plugin = $derived(pluginsStore.plugins.find((p) => p.handle === handle));
	const hasUpdate = $derived(
		plugin !== undefined &&
			plugin.enabled &&
			plugin.latest !== null &&
			plugin.latest !== plugin.version
	);

	let error = $state('');
	/** Rendered README; `null` for a plugin that ships none, `undefined` while loading. */
	let readme = $state<string | null | undefined>(undefined);

	onMount(() => {
		void pluginsStore.load();
	});

	$effect(() => {
		readme = undefined;
		api.plugins
			.readme(handle)
			.then((markdown) => (readme = DOMPurify.sanitize(marked.parse(markdown, { async: false }))))
			.catch(() => (readme = null));
	});

	let inputs = $state<InputSpec[]>([]);
	let values = $state<Record<string, unknown>>({});
	let saving = $state(false);
	let saved = $state(false);
	let settingsError = $state('');

	$effect(() => {
		inputs = [];
		settingsError = '';
		api.plugins
			.settings(handle)
			.then((settings) => {
				inputs = settings.inputs;
				values = Object.fromEntries(
					settings.inputs.map((spec) => [spec.name, settings.values[spec.name] ?? blankFor(spec)])
				);
			})
			.catch((e) => (settingsError = e instanceof Error ? e.message : String(e)));
	});

	async function saveSettings() {
		saving = true;
		saved = false;
		settingsError = '';
		try {
			await api.plugins.saveSettings(handle, values);
			saved = true;
		} catch (e) {
			settingsError = e instanceof Error ? e.message : String(e);
		}
		saving = false;
	}

	async function act(change: () => Promise<unknown>) {
		error = '';
		try {
			await change();
			await pluginsStore.reload();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function uninstall() {
		await act(() => api.plugins.uninstall(handle));
		if (!error) navigator.start().plugins();
	}

	function openLinksOutside(event: MouseEvent) {
		const link = (event.target as Element).closest('a');
		if (!link?.href) return;
		event.preventDefault();
		window.open(link.href, '_blank', 'noopener');
	}
</script>

{#if !pluginsStore.loaded}
	<div class="empty">Loading…</div>
{:else if !plugin}
	<div class="empty">This plugin isn't installed.</div>
{:else}
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
				<button
					type="button"
					class="outline"
					class:on={plugin.enabled}
					onclick={() => act(() => api.plugins.toggle(handle, !plugin.enabled))}
				>
					{#if plugin.enabled}<Check size={14} /> Enabled{:else}Enable{/if}
				</button>
				{#if hasUpdate}
					<button
						type="button"
						class="outline"
						onclick={() => act(() => api.plugins.update(handle))}
					>
						Update to {plugin.latest}
					</button>
				{/if}
				<button
					type="button"
					class="icon"
					aria-label="Remove {plugin.name} for everyone"
					title="Remove for everyone"
					onclick={uninstall}
				>
					<Trash2 size={16} />
				</button>
			</div>
			{#if error}<p class="error">{error}</p>{/if}

			{#if inputs.length}
				<h3>Settings</h3>
				{#each inputs as spec (spec.name)}
					<div class="field">
						<span class="label">{spec.name}</span>
						<InputControl
							ty={spec}
							value={values[spec.name]}
							ariaLabel={spec.name}
							onchange={(next) => {
								values[spec.name] = next;
								saved = false;
							}}
						/>
					</div>
				{/each}
				<div class="actions">
					<button type="button" class="outline" onclick={saveSettings} disabled={saving}>
						{saving ? 'Saving…' : 'Save settings'}
					</button>
					{#if saved}<span class="muted">Saved</span>{/if}
				</div>
			{/if}
			{#if settingsError}<p class="error">{settingsError}</p>{/if}

			<h3>Permissions</h3>
			{#if plugin.permissions.length}
				<table>
					<tbody>
						{#each plugin.permissions as permission (permission)}
							<tr>
								<td>{permissionLabels[permission] ?? permission}</td>
								<td class="muted"><code>{permission}</code></td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="muted">None.</p>
			{/if}
		</aside>

		<section class="readme">
			{#if readme === undefined}
				<p class="muted">Loading…</p>
			{:else if readme === null}
				<p class="muted">This plugin has no README.</p>
			{:else}
				<!-- Sanitized with DOMPurify. -->
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				<div onclickcapture={openLinksOutside}>{@html readme}</div>
			{/if}
		</section>
	</div>
{/if}

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
	.muted {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.label {
		font-family: var(--font-family-mono);
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
		align-items: center;
		gap: var(--spacing-sm);
	}
	.outline {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: none;
		border: 1px solid var(--color-border-medium);
	}
	.outline.on {
		color: var(--color-success-text);
		border-color: var(--color-success-border);
	}
	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		margin-left: auto;
		background: none;
		color: var(--color-text-secondary);
	}
	.icon:hover {
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

	.readme {
		padding: var(--spacing-lg);
		overflow-y: auto;
	}
	.readme :global(:is(h1, h2, h3)) {
		margin: var(--spacing-sm) 0 var(--spacing-xs);
		font-size: var(--font-size-h4);
	}
	.readme :global(:is(p, ul, ol, pre)) {
		margin: 0 0 var(--spacing-sm);
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

	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
	}
</style>
