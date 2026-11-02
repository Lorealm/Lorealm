<script lang="ts">
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { usePlatform } from '$lib/api/client.js';
	import { BUILT_IN_STYLES, CLASSIC_NAME } from '$lib/assets/styles/builtin.js';
	import NumberInput from '$lib/components/NumberInput.svelte';
	import { fitImageForStorage } from '$lib/assets/scripts/image.js';
	import { backdrop } from '$lib/stores/backdrop.svelte.js';
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { showError, showToast } from '$lib/stores/errorToast.svelte.js';
	import {
		parseStyle,
		styleFile,
		styleFilename,
		styles,
		tokenCatalogue
	} from '$lib/stores/styles.svelte.js';
	import { APPEARANCE_GROUPS, FONT_CHOICES, type TokenField } from './appearanceTokens.js';
	import {
		Download,
		Image as ImageIcon,
		Plus,
		RotateCcw,
		Search,
		Trash2,
		Undo2,
		Upload,
		X
	} from '@lucide/svelte';

	const FILE = { label: 'Lorealm style', extensions: ['json'] };
	const IMAGE = { label: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'avif'] };

	const shipped = tokenCatalogue();

	let query = $state('');

	const groups = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return APPEARANCE_GROUPS;

		return APPEARANCE_GROUPS.map((group) => ({
			...group,
			fields: group.fields.filter((f) => f.label.toLowerCase().includes(q) || f.name.includes(q))
		})).filter((group) => group.fields.length > 0);
	});

	const overridden = $derived(Object.keys(styles.tokens).length);

	function effective(field: TokenField): string {
		return styles.tokens[field.name] ?? shipped[field.name] ?? '';
	}

	function set(field: TokenField, value: string): void {
		if (value.trim() === '') styles.removeToken(field.name);
		else styles.setToken(field.name, value);
	}

	async function importStyle(): Promise<void> {
		try {
			const text = await usePlatform().files.open(FILE);
			if (text === null) return; // Nothing picked.

			const style = parseStyle(text);
			await styles.add(style);
			showToast({ message: `“${style.name}” is now in force.`, title: 'Style imported' });
		} catch (e) {
			showError(e, { title: 'Could not import that style' });
		}
	}

	async function chooseBackdrop(): Promise<void> {
		try {
			const picked = await usePlatform().files.openBinary(IMAGE);
			if (picked === null) return; // Nothing picked.
			await backdrop.set(await fitImageForStorage(picked));
		} catch (e) {
			showError(e, { title: 'Could not load that image' });
		}
	}

	async function exportStyle(): Promise<void> {
		const style = styles.active;
		if (!style) return;

		try {
			await usePlatform().files.save(styleFile(style), {
				...FILE,
				suggestedName: styleFilename(style)
			});
		} catch (e) {
			showError(e, { title: 'Could not export that style' });
		}
	}

	let canvas: CanvasRenderingContext2D | null | undefined;

	function parseColor(value: string): { hex: string; alpha: number } | null {
		canvas ??= document.createElement('canvas').getContext('2d');
		if (!canvas || !value.trim()) return null;

		canvas.fillStyle = '#000000';
		canvas.fillStyle = value;
		const reading = canvas.fillStyle;
		canvas.fillStyle = '#ffffff';
		canvas.fillStyle = value;
		if (canvas.fillStyle !== reading) return null;

		if (reading.startsWith('#')) return { hex: reading, alpha: 1 };

		const [r, g, b, alpha = 1] = (reading.match(/[\d.]+/g) ?? []).map(Number);
		const channels = [r, g, b].map((c) => Math.round(c).toString(16).padStart(2, '0'));
		return { hex: `#${channels.join('')}`, alpha };
	}

	function withAlpha(picked: string, alpha: number): string {
		if (alpha >= 1) return picked;
		const [r, g, b] = [1, 3, 5].map((i) => parseInt(picked.slice(i, i + 2), 16));
		return `rgba(${r}, ${g}, ${b}, ${alpha})`;
	}

	function numeric(value: string | undefined): number | '' {
		const parsed = parseFloat(value ?? '');
		return Number.isNaN(parsed) ? '' : parsed;
	}
</script>

{#snippet control(field: TokenField)}
	{@const override = styles.tokens[field.name]}

	{#if field.kind === 'color'}
		{@const color = parseColor(effective(field))}
		{#if color}
			<input
				class="swatch"
				type="color"
				value={color.hex}
				aria-label="{field.label} colour"
				oninput={(e) => set(field, withAlpha(e.currentTarget.value, color.alpha))}
			/>
		{:else}
			<span class="swatch empty" {@attach tooltip('Not a colour the picker can open')}></span>
		{/if}
		<input
			class="text"
			value={override ?? ''}
			placeholder={shipped[field.name] ?? ''}
			spellcheck="false"
			oninput={(e) => set(field, e.currentTarget.value)}
		/>
	{:else if field.kind === 'px'}
		<NumberInput
			min={0}
			step={1}
			width="7rem"
			ariaLabel={field.label}
			value={numeric(override)}
			placeholder={String(numeric(shipped[field.name]))}
			onchange={(next) => set(field, next === '' ? '' : `${next}px`)}
		/>
		<span class="unit">px</span>
	{:else if field.kind === 'font'}
		{@const current = effective(field)}
		<select class="font" value={current} onchange={(e) => set(field, e.currentTarget.value)}>
			{#each FONT_CHOICES as choice (choice.value)}
				<option value={choice.value}>{choice.label}</option>
			{/each}
			{#if !FONT_CHOICES.some((c) => c.value === current)}
				<option value={current}>{current}</option>
			{/if}
		</select>
	{:else}
		<input
			class="text"
			value={override ?? ''}
			placeholder={shipped[field.name] ?? ''}
			spellcheck="false"
			oninput={(e) => set(field, e.currentTarget.value)}
		/>
	{/if}
{/snippet}

<section class="section">
	<div class="body">
		<h2>Appearance</h2>

		<div class="library">
			<select
				class="picker"
				aria-label="Style"
				value={styles.activeId ?? ''}
				onchange={(e) => styles.select(e.currentTarget.value === '' ? null : e.currentTarget.value)}
			>
				<optgroup label="Built in">
					<option value="">{CLASSIC_NAME}</option>
					{#each BUILT_IN_STYLES as style (style.id)}
						<option value={style.id}>{style.name}</option>
					{/each}
				</optgroup>
				{#if styles.all.length > 0}
					<optgroup label="Yours">
						{#each styles.all as style (style.id)}
							<option value={style.id}>{style.name}</option>
						{/each}
					</optgroup>
				{/if}
			</select>
			<button class="action" onclick={() => styles.create()}>
				<Plus size={14} />
				New
			</button>
			<button class="action" onclick={importStyle}>
				<Upload size={14} />
				Import
			</button>
			<button class="action" disabled={!styles.active} onclick={exportStyle}>
				<Download size={14} />
				Export
			</button>
		</div>

		{#if styles.active && !styles.editable}
			<p class="hint">
				<code>{styles.active.name}</code> ships with the client. Changing anything below copies it into
				a new style.
			</p>
		{/if}

		{#if styles.editable && styles.active}
			{@const active = styles.active}
			<div class="library">
				<input
					class="name"
					aria-label="Style name"
					value={active.name}
					oninput={(e) => styles.rename(e.currentTarget.value)}
				/>
				<input
					class="author"
					aria-label="Style author"
					placeholder="The author's name"
					value={active.author ?? ''}
					oninput={(e) => styles.setAuthor(e.currentTarget.value)}
				/>
				<button
					class="action"
					{@attach tooltip('Delete this style')}
					onclick={() => styles.remove(active.id)}
				>
					<Trash2 size={14} />
					Delete
				</button>
			</div>
		{/if}

		<h3>Backdrop</h3>
		<div class="library">
			{#if backdrop.image}
				<span class="preview">
					<TransportImage
						src={backdrop.image}
						alt="The current backdrop"
						style="display:block;width:100%;height:100%"
					/>
				</span>
			{/if}
			<button class="action" onclick={chooseBackdrop}>
				<ImageIcon size={14} />
				{backdrop.image ? 'Replace image' : 'Choose image'}
			</button>
			{#if backdrop.image}
				<button class="action" onclick={() => backdrop.clear()}>
					<X size={14} />
					Remove
				</button>
			{/if}
		</div>

		<div class="toolbar">
			<div class="searchRow">
				<Search size={14} />
				<input class="search" placeholder="Search appearance settings" bind:value={query} />
			</div>
			<span class="count">
				{overridden === 0 ? 'Nothing overridden' : `${overridden} overridden`}
			</span>
			<button
				class="reset"
				disabled={overridden === 0 || !styles.editable}
				onclick={() => styles.reset()}
			>
				<RotateCcw size={14} />
				Reset all
			</button>
		</div>

		{#each groups as group (group.title)}
			<div class="group">
				<h3>{group.title}</h3>
				<p class="hint">{group.blurb}</p>

				{#each group.fields as field (field.name)}
					<div class="row" class:changed={field.name in styles.tokens}>
						<span class="label">
							{field.label}
							<span class="token">{field.name}</span>
						</span>
						<span class="control">{@render control(field)}</span>
						<button
							class="revert"
							{@attach tooltip('Back to the shipped value')}
							disabled={!(field.name in styles.tokens)}
							onclick={() => styles.removeToken(field.name)}
						>
							<Undo2 size={14} />
						</button>
					</div>
				{/each}
			</div>
		{:else}
			<span class="empty">Nothing matches “{query}”.</span>
		{/each}

		<div class="group">
			<h3>Custom CSS</h3>

			<textarea
				class="css"
				rows="14"
				spellcheck="false"
				placeholder={'.app .message-bubble {\n\tborder-radius: 2px;\n}'}
				value={styles.css}
				oninput={(e) => styles.setCss(e.currentTarget.value)}
			></textarea>
		</div>
	</div>
</section>

<style>
	.section {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		flex-direction: column;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
	}
	.body h2 {
		font-size: var(--font-size-h4);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		max-width: 46em;
		margin: 0;
	}
	code {
		font-family: var(--font-family-mono);
		color: var(--color-text-secondary);
	}
	.library,
	.toolbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		max-width: 44em;
	}
	.picker {
		flex: 1 1 auto;
		width: auto;
	}
	.name {
		flex: 0 0 12em;
	}
	.author {
		flex: 1 1 auto;
		width: auto;
		min-width: 0;
	}
	.preview {
		flex: 0 0 auto;
		width: 84px;
		height: 48px;
		overflow: hidden;
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
	}
	.action {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
		padding: 6px 12px;
		font-size: var(--font-size-sm);
	}
	.action:disabled {
		color: var(--color-text-disabled);
		cursor: default;
		background-color: var(--color-bg-elevated);
	}
	.searchRow {
		display: flex;
		align-items: center;
		flex: 1 1 auto;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
	}
	.searchRow:focus-within {
		border-color: var(--color-border-active);
	}
	.search {
		background: transparent;
		border: none;
		padding: 6px 0;
		font-size: var(--font-size-sm);
	}
	.count {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.reset {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
		padding: 6px 12px;
		font-size: var(--font-size-sm);
	}
	.reset:disabled {
		color: var(--color-text-disabled);
		cursor: default;
		background-color: var(--color-bg-elevated);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		max-width: 44em;
	}
	.group h3 {
		margin-bottom: 2px;
	}
	.group .hint {
		margin-bottom: var(--spacing-xs);
	}
	.row {
		display: grid;
		grid-template-columns: 14em 1fr auto;
		align-items: center;
		gap: var(--spacing-sm);
		padding: 3px var(--spacing-sm);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
	}
	.row:hover {
		background-color: var(--color-bg-elevated-2);
	}
	.row.changed {
		border-color: var(--color-border-medium);
	}
	.label {
		display: flex;
		flex-direction: column;
		min-width: 0;
		font-size: var(--font-size-sm);
	}
	.token {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.control {
		display: flex;
		align-items: stretch;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.swatch {
		flex: 0 0 auto;
		width: 34px;
		padding: 0;
		background: none;
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}
	.swatch.empty {
		background-image: linear-gradient(
			135deg,
			transparent 45%,
			var(--color-border-strong) 45%,
			var(--color-border-strong) 55%,
			transparent 55%
		);
		cursor: default;
	}
	.text {
		flex: 1 1 auto;
		width: auto;
		min-width: 0;
		font-family: var(--font-family-mono);
	}
	.unit {
		display: flex;
		align-items: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.font {
		flex: 1 1 auto;
		width: auto;
	}
	.revert {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 3px;
		background: transparent;
		color: var(--color-text-secondary);
	}
	.revert:disabled {
		color: var(--color-text-disabled);
		cursor: default;
		background: transparent;
	}
	.css {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-snug);
		resize: vertical;
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
