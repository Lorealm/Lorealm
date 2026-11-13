<script lang="ts" module>
	import type { InputSpecType } from '#lib/types/blueprint.js';

	export function blankFor(ty: InputSpecType): unknown {
		switch (ty.type) {
			case 'list':
				return [];
			case 'bool':
				return false;
			case 'range':
				return ty.min;
			case 'span':
				return { from: ty.min, to: ty.max };
			case 'object':
				return Object.fromEntries(ty.fields.map((f) => [f.name, blankFor(f)]));
			default:
				return '';
		}
	}
</script>

<script lang="ts">
	import { Plus, X } from '@lucide/svelte';
	import type { Attachment } from 'svelte/attachments';
	import NumberInput from '#lib/components/NumberInput.svelte';
	import { connectionStore } from '#lib/stores/connection.svelte.js';
	import { itemsOf } from './resources.svelte.js';
	import Self from './InputControl.svelte';

	let {
		ty,
		value,
		onchange,
		ariaLabel
	}: {
		ty: InputSpecType;
		value: unknown;
		onchange: (next: unknown) => void;
		ariaLabel: string;
	} = $props();

	const span = $derived((value ?? { from: 0, to: 0 }) as { from: number; to: number });

	/** Keep the handles from crossing, whichever one is being dragged. */
	function setSpan(from: number, to: number) {
		onchange({ from: Math.min(from, to), to: Math.max(from, to) });
	}

	/** Grows a text area to fit its text, as it's typed and when the value changes. */
	const grow: Attachment<HTMLTextAreaElement> = (area) => {
		// Read so the attachment re-runs when the value changes, such as once
		// saved settings load.
		void value;
		const fit = () => {
			area.style.height = 'auto';
			area.style.height = `${area.scrollHeight + area.offsetHeight - area.clientHeight}px`;
		};
		fit();
		area.addEventListener('input', fit);
		return () => area.removeEventListener('input', fit);
	};
</script>

{#if ty.type === 'list'}
	{@const rows = (value ?? []) as unknown[]}
	{@const kind = ty.of.type === 'reference' ? ty.of.kind : null}
	{#if kind}
		<div class="checks">
			{#each itemsOf(kind) as item (item.id)}
				<label>
					<input
						type="checkbox"
						checked={(rows as string[]).includes(item.id)}
						onchange={(e) =>
							onchange(
								e.currentTarget.checked
									? [...rows, item.id]
									: (rows as string[]).filter((held) => held !== item.id)
							)}
					/>
					<span>{item.name}</span>
				</label>
			{:else}
				<p class="none">You own no {kind}s to pick from.</p>
			{/each}
		</div>
	{:else}
		<div class="rows">
			{#each rows as row, i (i)}
				<div class="row">
					<Self
						ty={ty.of}
						value={row}
						ariaLabel="{ariaLabel} {i + 1}"
						onchange={(next) => onchange(rows.map((held, at) => (at === i ? next : held)))}
					/>
					<button
						type="button"
						class="drop"
						aria-label="Remove {ariaLabel} {i + 1}"
						onclick={() => onchange(rows.filter((_, at) => at !== i))}
					>
						<X size={12} />
					</button>
				</div>
			{/each}
			<button type="button" class="add" onclick={() => onchange([...rows, blankFor(ty.of)])}>
				<Plus size={12} /> Add
			</button>
		</div>
	{/if}
{:else if ty.type === 'reference' && ty.kind === null}
	{@const keys = connectionStore.secretsFor(ty.ref_name)}
	{#if keys.length}
		<select
			aria-label={ariaLabel}
			value={(value as string) ?? ''}
			onchange={(e) => onchange(e.currentTarget.value || null)}
		>
			<option value="">— pick a key —</option>
			{#each keys as key (key.id)}
				<option value={key.id}>{key.label} (…{key.key_hint})</option>
			{/each}
		</select>
	{:else}
		<p class="none">You have no {ty.ref_name} keys yet. Add one under Connections → API keys.</p>
	{/if}
{:else if ty.type === 'reference'}
	<select
		aria-label={ariaLabel}
		value={(value as string) ?? ''}
		onchange={(e) => onchange(e.currentTarget.value)}
	>
		<option value="">— pick one —</option>
		{#each ty.kind ? itemsOf(ty.kind) : [] as item (item.id)}
			<option value={item.id}>{item.name}</option>
		{/each}
	</select>
{:else if ty.type === 'range'}
	<div class="slider">
		<input
			type="range"
			min={ty.min}
			max={ty.max}
			step={(ty.max - ty.min) / 100}
			aria-label={ariaLabel}
			value={(value as number) ?? ty.min}
			oninput={(e) => onchange(Number(e.currentTarget.value))}
		/>
		<output>{value ?? ty.min}</output>
	</div>
{:else if ty.type === 'span'}
	<div class="slider">
		<div class="handles">
			<input
				type="range"
				min={ty.min}
				max={ty.max}
				step={(ty.max - ty.min) / 100}
				aria-label="{ariaLabel} from"
				value={span.from}
				oninput={(e) => setSpan(Number(e.currentTarget.value), span.to)}
			/>
			<input
				type="range"
				min={ty.min}
				max={ty.max}
				step={(ty.max - ty.min) / 100}
				aria-label="{ariaLabel} to"
				value={span.to}
				oninput={(e) => setSpan(span.from, Number(e.currentTarget.value))}
			/>
		</div>
		<output>{span.from} – {span.to}</output>
	</div>
{:else if ty.type === 'enum'}
	<select
		aria-label={ariaLabel}
		value={(value as string) ?? ''}
		onchange={(e) => onchange(e.currentTarget.value)}
	>
		<option value="">— default —</option>
		{#each ty.variants as variant (variant)}
			<option value={variant}>{variant}</option>
		{/each}
	</select>
{:else if ty.type === 'bool'}
	<label class="check">
		<input
			type="checkbox"
			checked={value === true}
			aria-label={ariaLabel}
			onchange={(e) => onchange(e.currentTarget.checked)}
		/>
		<span>Yes</span>
	</label>
{:else if ty.type === 'number'}
	<NumberInput
		{ariaLabel}
		value={(value as number | '') ?? ''}
		onchange={(next) => onchange(next)}
	/>
{:else if ty.type === 'object'}
	{@const held = (value ?? {}) as Record<string, unknown>}
	<div class="group">
		{#each ty.fields as field (field.name)}
			<label>
				<span>{field.name}</span>
				<Self
					ty={field}
					value={held[field.name]}
					ariaLabel="{ariaLabel} {field.name}"
					onchange={(next) => onchange({ ...held, [field.name]: next })}
				/>
			</label>
		{/each}
	</div>
{:else}
	<textarea
		class="text"
		rows="1"
		aria-label={ariaLabel}
		value={(value as string) ?? ''}
		{@attach grow}
		onchange={(e) => onchange(e.currentTarget.value)}
	></textarea>
{/if}

<style>
	.text {
		resize: none;
		overflow: hidden;
	}
	.slider {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.slider input[type='range'] {
		flex: 1 1 auto;
		min-width: 0;
		padding: 0;
	}
	.handles {
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	output {
		flex: 0 0 auto;
		min-width: 4.5em;
		text-align: right;
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.checks {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 12em;
		overflow-y: auto;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.checks label {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.none {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.rows {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.row {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.row > :global(*:first-child) {
		flex: 1 1 auto;
		min-width: 0;
	}
	.drop {
		flex: 0 0 auto;
		display: flex;
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.drop:hover {
		color: var(--color-error-text);
	}
	.add {
		align-self: flex-start;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 2px var(--spacing-sm);
		background: transparent;
		border: 1px dashed var(--color-border-medium);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.add:hover {
		border-color: var(--color-border-active);
		color: var(--color-text-accent);
	}

	.check {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		border-left: 2px solid var(--color-border-medium);
	}
	.group label {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.group label > span {
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
</style>
