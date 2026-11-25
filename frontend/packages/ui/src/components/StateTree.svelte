<script lang="ts">
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import { ChevronRight } from '@lucide/svelte';
	import { SvelteSet } from 'svelte/reactivity';

	let {
		data,
		openDepth = 1
	}: {
		/** The object whose keys form the top level of the tree. */
		data: Record<string, unknown>;
		/** Branches shallower than this start expanded. */
		openDepth?: number;
	} = $props();

	const LONG = 60;

	/** Paths whose open state differs from the depth default. */
	const flipped = new SvelteSet<string>();
	/** Long strings the reader has unfolded. */
	const unfolded = new SvelteSet<string>();

	$effect(() => {
		void data;
		flipped.clear();
		unfolded.clear();
	});

	type Kind = 'object' | 'array' | 'string' | 'number' | 'boolean' | 'null';

	function kindOf(value: unknown): Kind {
		if (value === null || value === undefined) return 'null';
		if (Array.isArray(value)) return 'array';
		const t = typeof value;
		if (t === 'object') return 'object';
		if (t === 'string' || t === 'number' || t === 'boolean') return t;
		return 'string';
	}

	function children(value: unknown): Array<[string, unknown]> {
		if (Array.isArray(value)) return value.map((v, i) => [String(i), v]);
		return Object.entries(value as Record<string, unknown>).sort(([a], [b]) => a.localeCompare(b));
	}

	function childPath(parent: string, key: string, parentIsArray: boolean): string {
		if (parentIsArray) return `${parent}[${key}]`;
		return parent ? `${parent}.${key}` : key;
	}

	function isOpen(path: string, depth: number): boolean {
		return depth < openDepth !== flipped.has(path);
	}

	function toggle(path: string) {
		if (flipped.has(path)) flipped.delete(path);
		else flipped.add(path);
	}

	function toggleUnfold(path: string) {
		if (unfolded.has(path)) unfolded.delete(path);
		else unfolded.add(path);
	}

	function isLong(s: string): boolean {
		return s.length > LONG || s.includes('\n');
	}

	function scalar(value: unknown): string {
		if (value === null || value === undefined) return String(value);
		if (typeof value === 'string') return JSON.stringify(value);
		return String(value);
	}

	let roots = $derived(children(data));
</script>

{#snippet node(key: string, value: unknown, path: string, depth: number, inArray: boolean)}
	{@const kind = kindOf(value)}
	{@const branch = kind === 'object' || kind === 'array'}
	{@const items = branch ? children(value) : []}
	<li class="node">
		{#if branch && items.length > 0}
			{@const open = isOpen(path, depth)}
			<button class="line branch" aria-expanded={open} onclick={() => toggle(path)}>
				<span class="chev" class:open><ChevronRight /></span>
				<span class="key" class:index={inArray} {@attach tooltip(path)}>{key}</span>
				<span class="summary">
					{kind === 'array' ? `[${items.length}]` : `{${items.length}}`}
				</span>
			</button>
			{#if open}
				<ul class="children">
					{#each items as [k, v] (k)}
						{@render node(k, v, childPath(path, k, kind === 'array'), depth + 1, kind === 'array')}
					{/each}
				</ul>
			{/if}
		{:else}
			<div class="line leaf">
				<span class="spacer"></span>
				<span class="key" class:index={inArray} {@attach tooltip(path)}>{key}</span>
				{#if branch}
					<span class="val v-null">{kind === 'array' ? '[]' : '{}'}</span>
				{:else if kind === 'string' && isLong(value as string)}
					{@const full = unfolded.has(path)}
					<button
						class="val v-string long"
						class:full
						aria-expanded={full}
						onclick={() => toggleUnfold(path)}>{full ? value : scalar(value)}</button
					>
				{:else}
					<span class="val v-{kind}">{scalar(value)}</span>
				{/if}
			</div>
		{/if}
	</li>
{/snippet}

<ul class="tree">
	{#each roots as [k, v] (k)}
		{@render node(k, v, k, 0, false)}
	{/each}
</ul>

<style>
	.tree,
	.children {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.tree {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
	}
	.children {
		margin-left: 6px;
		padding-left: var(--spacing-sm);
		border-left: 1px solid var(--color-border-subtle);
	}

	.line {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-xs);
		min-width: 0;
		padding: 2px var(--spacing-xs) 2px 0;
		border-radius: var(--radius-sm);
	}
	.leaf {
		flex-wrap: wrap;
	}
	.branch {
		width: 100%;
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.branch:hover {
		background: var(--color-bg-elevated-3);
	}

	.chev,
	.spacer {
		flex: 0 0 auto;
		display: inline-flex;
		align-self: center;
		width: 12px;
	}
	.chev {
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev.open {
		transform: rotate(90deg);
	}
	.chev :global(svg) {
		width: 12px;
		height: 12px;
	}

	.key {
		flex: 0 1 auto;
		min-width: 0;
		max-width: 50%;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-accent);
	}
	.key::after {
		content: ':';
		color: var(--color-text-muted);
	}
	.branch .key::after {
		content: '';
	}
	.key.index {
		color: var(--color-text-muted);
	}

	.summary {
		color: var(--color-text-muted);
	}

	.val {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
	}
	.v-string {
		color: var(--color-success-text);
	}
	.v-number {
		color: var(--color-info-text);
	}
	.v-boolean {
		color: var(--color-warning-text);
	}
	.v-null {
		color: var(--color-text-muted);
		font-style: italic;
	}

	.long {
		padding: 0;
		background: transparent;
		border-radius: 0;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.long:hover {
		text-decoration: underline dotted;
	}
	.long.full {
		flex-basis: 100%;
		margin-left: calc(12px + var(--spacing-xs));
		max-height: 200px;
		overflow-y: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		text-decoration: none;
	}
</style>
