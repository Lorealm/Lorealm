<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import {
		paramDocs,
		processorKey,
		weaverVocabularyStore
	} from '#lib/stores/weaverVocabulary.svelte.js';
	import type { ParamDef, PropertyDef } from '#lib/types/weaver.js';
	import { renderDoc } from '#lib/weaver/editor/weaver-render.js';
	import { Check, ChevronDown, Copy } from '@lucide/svelte';

	let collapsed = $state(false);
	let query = $state('');
	let open = $state<string | null>(null);
	let copied = $state<string | null>(null);

	$effect(() => {
		void weaverVocabularyStore.load();
	});

	function params(list: Array<ParamDef | PropertyDef>) {
		return list.map((p) => ({
			name: 'name' in p ? p.name : p.key,
			optional: !p.required,
			type: p.expected_type ?? 'any'
		}));
	}

	const entries = $derived(
		[
			...weaverVocabularyStore.commands.map((cmd) => ({
				sigil: '$',
				name: cmd.name,
				params: params(cmd.params),
				returns: cmd.returns,
				description: cmd.description,
				docs: paramDocs(cmd)
			})),
			...weaverVocabularyStore.processors.map((proc) => ({
				sigil: '@',
				name: processorKey(proc),
				params: params(proc.properties),
				returns: proc.returns,
				description: proc.description,
				docs: paramDocs(proc)
			}))
		]
			.map((e) => ({ ...e, id: `${e.sigil}[${e.name}]` }))
			.sort((a, b) => a.id.localeCompare(b.id))
	);

	const shown = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (!needle) return entries;
		return entries.filter(
			(e) => e.id.toLowerCase().includes(needle) || e.description.toLowerCase().includes(needle)
		);
	});

	async function copy(entry: { id: string; sigil: string; name: string }) {
		await navigator.clipboard.writeText(`${entry.sigil}[${entry.name}()]`);
		copied = entry.id;
		setTimeout(() => {
			if (copied === entry.id) copied = null;
		}, 1200);
	}

	function onRowKeydown(e: KeyboardEvent, entry: { id: string; sigil: string; name: string }) {
		if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'c') return;
		if (!window.getSelection()?.isCollapsed) return;
		e.preventDefault();
		void copy(entry);
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">{t.inspector.widgets.weaverReference}</span>
		<span class="count">{entries.length}</span>
	</button>

	{#if !collapsed}
		<input
			class="search"
			type="search"
			placeholder={t.inspector.weaverReference.search}
			aria-label={t.inspector.weaverReference.search}
			bind:value={query}
		/>
		<div class="list">
			{#each shown as entry (entry.id)}
				<div class="row" class:open={open === entry.id}>
					<button
						class="toggle"
						aria-expanded={open === entry.id}
						onclick={() => (open = open === entry.id ? null : entry.id)}
						onkeydown={(e) => onRowKeydown(e, entry)}
					>
						<span class="bracket">{entry.sigil}[</span><span class="fn">{entry.name}</span><span
							class="bracket">]</span
						>
					</button>
					<button
						class="copy"
						title={t.inspector.weaverReference.copy}
						aria-label={t.inspector.weaverReference.copy}
						onclick={() => copy(entry)}
					>
						{#if copied === entry.id}<Check />{:else}<Copy />{/if}
					</button>
				</div>
				{#if open === entry.id}
					<div class="doc">
						<div class="sig">
							<span class="fn">{entry.name}</span><span class="punct">(</span
							>{#each entry.params as param, i (param.name)}{#if i > 0}<span class="punct"
										>,&nbsp;</span
									>{/if}<span class="param">{param.name}</span><span class="punct"
									>{param.optional ? '?' : ''}:&nbsp;</span
								><span class="type">{param.type}</span>{/each}<span class="punct">)</span>
							<span class="punct">-&gt;</span> <span class="type">{entry.returns}</span>
						</div>
						{#if entry.description}
							<!-- eslint-disable-next-line svelte/no-at-html-tags -->
							<div class="body">{@html renderDoc(entry.description)}</div>
						{/if}
						{#if entry.docs.length > 0}
							<dl>
								{#each entry.docs as param (param.name)}
									<dt>{param.name}</dt>
									<!-- eslint-disable-next-line svelte/no-at-html-tags -->
									<dd>{@html renderDoc(param.description)}</dd>
								{/each}
							</dl>
						{/if}
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-large, 340px);
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
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.count {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
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

	.search {
		flex: 0 0 auto;
		margin: var(--spacing-sm) var(--spacing-md) 0;
		font-size: var(--font-size-xs);
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-xs) 0;
	}
	.row {
		display: flex;
		align-items: center;
	}
	.row:hover,
	.row.open {
		background-color: var(--color-hover);
	}
	.toggle {
		flex: 1;
		min-width: 0;
		padding: 2px var(--spacing-md);
		background: transparent;
		border-radius: 0;
		text-align: left;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.copy {
		flex: 0 0 auto;
		display: inline-flex;
		padding: 2px var(--spacing-md);
		background: transparent;
		border-radius: 0;
		color: var(--color-text-muted);
	}
	.copy:hover {
		color: var(--color-text-primary);
	}
	.copy :global(svg) {
		width: 12px;
		height: 12px;
	}

	.bracket {
		color: var(--weaver-color-bracket);
		font-weight: 600;
	}
	.fn {
		color: var(--weaver-color-function);
	}
	.param {
		color: var(--weaver-color-property);
	}
	.type {
		color: var(--weaver-color-type);
	}
	.punct {
		color: var(--weaver-color-punctuation);
	}

	.doc {
		padding: var(--spacing-xs) var(--spacing-md) var(--spacing-sm);
		font-size: var(--font-size-xs);
		color: var(--color-text-secondary);
	}
	.sig {
		font-family: var(--font-family-mono);
		overflow-wrap: anywhere;
	}
	.body :global(p),
	dd :global(p) {
		margin: var(--spacing-xs) 0 0;
	}
	dl {
		margin: var(--spacing-xs) 0 0;
	}
	dt {
		font-family: var(--font-family-mono);
		color: var(--weaver-color-property);
		margin-top: var(--spacing-xs);
	}
	dd {
		margin: 0 0 0 var(--spacing-md);
	}
	dd :global(p:first-child) {
		margin-top: 0;
	}
</style>
