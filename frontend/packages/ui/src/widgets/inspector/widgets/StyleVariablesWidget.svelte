<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { frameTheme } from '#lib/stores/frameTheme.svelte.js';
	import { rootVariableEntries } from '#lib/core/frameStyle.js';
	import { ChevronDown } from '@lucide/svelte';

	let collapsed = $state(false);
	let copied = $state<string | null>(null);
	let query = $state('');

	type Kind = 'color' | 'shadow' | 'radius' | 'font-family' | 'font-weight' | null;

	function kindOf(name: string, value: string): Kind {
		if (value.includes('gradient(') || CSS.supports('color', value)) return 'color';
		if (name.includes('shadow') && CSS.supports('box-shadow', value)) return 'shadow';
		if (name.includes('radius')) return 'radius';
		if (name.startsWith('--font-family')) return 'font-family';
		if (name.startsWith('--font-weight')) return 'font-weight';
		return null;
	}

	const variables = $derived.by(() => {
		void frameTheme.css;
		return rootVariableEntries()
			.sort(([a], [b]) => a.localeCompare(b))
			.map(([name, value]) => ({ name, value, kind: kindOf(name, value) }));
	});

	const shown = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		if (!needle) return variables;
		return variables.filter(
			(v) => v.name.toLowerCase().includes(needle) || v.value.toLowerCase().includes(needle)
		);
	});

	async function copy(name: string) {
		await navigator.clipboard.writeText(`var(${name})`);
		copied = name;
		setTimeout(() => {
			if (copied === name) copied = null;
		}, 1200);
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">{t.inspector.widgets.styleVariables}</span>
		<span class="count">{variables.length}</span>
	</button>

	{#if !collapsed}
		<input
			class="search"
			type="search"
			placeholder={t.inspector.styleVariables.search}
			aria-label={t.inspector.styleVariables.search}
			bind:value={query}
		/>
		<div class="list">
			{#each shown as { name, value, kind } (name)}
				<button class="row" onclick={() => copy(name)}>
					<span class="preview">
						{#if kind === 'color'}
							<span class="swatch" style:background="var({name})"></span>
						{:else if kind === 'shadow'}
							<span class="swatch" style:box-shadow="var({name})"></span>
						{:else if kind === 'radius'}
							<span class="swatch corner" style:border-top-left-radius="var({name})"></span>
						{:else if kind === 'font-family'}
							<span class="sample" style:font-family="var({name})">Aa</span>
						{:else if kind === 'font-weight'}
							<span class="sample" style:font-weight="var({name})">Aa</span>
						{/if}
					</span>
					<span class="name">{name}</span>
					<span class="value">{copied === name ? t.inspector.styleVariables.copied : value}</span>
				</button>
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
		gap: var(--spacing-sm);
		width: 100%;
		padding: 2px var(--spacing-md);
		background: transparent;
		border-radius: 0;
		text-align: left;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.row:hover {
		background-color: var(--color-hover);
	}
	.preview {
		flex: 0 0 14px;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 14px;
	}
	.swatch {
		width: 12px;
		height: 12px;
		border: 1px solid var(--color-border-subtle);
		border-radius: 2px;
	}
	.swatch.corner {
		border-width: 2px 0 0 2px;
		border-color: var(--color-text-muted);
		border-radius: 0;
	}
	.sample {
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		line-height: 1;
	}
	.name {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
	}
	.value {
		flex: 1 1 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		text-align: right;
		color: var(--color-text-muted);
	}
</style>
