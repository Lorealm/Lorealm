<script lang="ts" module>
	export const COMFORTABLE_DEPTH = 2;
</script>

<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { Plus, Trash2, TriangleAlert } from '@lucide/svelte';
	import { tick, untrack } from 'svelte';
	import NumberInput from '#lib/components/NumberInput.svelte';
	import {
		baseOf,
		blankField,
		declaredText,
		holdsToBase,
		holdsValue,
		isList,
		TYPE_NAME,
		withList,
		type InputField
	} from './model.js';
	import { RESOURCE_KINDS } from './resources.svelte.js';
	import Self from './ObjectFields.svelte';

	let {
		fields,
		depth = 0,
		ping = null,
		onchange
	}: {
		fields: InputField[];
		/** How many tables deep this one sits */
		depth?: number;
		ping?: { path: string[] } | null;
		onchange: (next: InputField[]) => void;
	} = $props();

	/** Which field's settings are open, by index. */
	let open = $state<number | null>(null);
	let nameEls = $state<HTMLInputElement[]>([]);

	$effect(() => {
		const path = ping?.path;
		if (!path) return;
		const want = path[depth];
		if (want === undefined) return;
		const at = untrack(() => fields).findIndex((f) => f.name === want);
		if (at < 0) return;
		open = at;
		if (path.length === depth + 1) tick().then(() => nameEls[at]?.select());
	});

	function set(index: number, patch: Partial<InputField>) {
		onchange(fields.map((f, at) => (at === index ? { ...f, ...patch } : f)));
	}

	function remove(index: number) {
		if (open !== null && open >= index) open = open === index ? null : open - 1;
		onchange(fields.filter((_, at) => at !== index));
	}

	const splitVariants = (text: string) =>
		text
			.split(',')
			.map((v) => v.trim())
			.filter(Boolean);
</script>

<div class="fields">
	<span class="head">
		{depth === 0 ? t.realms.fields.title : t.realms.fields.nested}
		<button
			type="button"
			aria-label={t.realms.fields.add}
			onclick={() => onchange([...fields, blankField()])}
		>
			<Plus size={12} />
		</button>
	</span>

	{#if depth > COMFORTABLE_DEPTH}
		<p class="deep">
			<TriangleAlert size={12} />
			{t.realms.fields.tooDeep(depth)}
		</p>
	{/if}

	<table>
		<thead>
			<tr>
				<th>{t.realms.fields.name}</th>
				<th>{t.realms.fields.holds}</th>
				<th class="checkCol">{t.realms.fields.list}</th>
				<th></th>
			</tr>
		</thead>
		<tbody>
			{#each fields as field, i (i)}
				<tr class:selected={open === i} onfocusin={() => (open = i)}>
					<td>
						<input
							class="mono"
							bind:this={nameEls[i]}
							value={field.name}
							placeholder={t.realms.fields.namePlaceholder}
							spellcheck="false"
							aria-label={t.realms.fields.nameAria(i + 1)}
							onchange={(e) => set(i, { name: e.currentTarget.value })}
						/>
					</td>
					<td>
						<select
							aria-label={t.realms.fields.typeAria(i + 1)}
							value={holdsValue(field.ty)}
							onchange={(e) =>
								set(i, { ty: withList(holdsToBase(e.currentTarget.value), isList(field.ty)) })}
						>
							<option value="string">{t.realms.fields.types.string}</option>
							<option value="number">{t.realms.fields.types.number}</option>
							<option value="bool">{t.realms.fields.types.bool}</option>
							<option value="enum">{t.realms.fields.types.enum}</option>
							<option value="range">{t.realms.fields.types.range}</option>
							<option value="span">{t.realms.fields.types.span}</option>
							<option value="object">{t.realms.fields.types.object}</option>
							{#each RESOURCE_KINDS as kind (kind)}
								<option value="ref:{kind}">{TYPE_NAME[kind]}</option>
							{/each}
						</select>
					</td>
					<td class="checkCol">
						<input
							type="checkbox"
							aria-label={t.realms.fields.listAria(i + 1)}
							checked={isList(field.ty)}
							onchange={(e) => set(i, { ty: withList(baseOf(field.ty), e.currentTarget.checked) })}
						/>
					</td>
					<td>
						<button
							type="button"
							class="remove"
							aria-label={t.realms.fields.removeAria(i + 1)}
							onclick={() => remove(i)}
						>
							<Trash2 size={12} />
						</button>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>

	{#if open !== null && fields[open]}
		{@const at = open}
		{@const field = fields[at]}
		{@const inner = baseOf(field.ty)}
		<div class="detail">
			<span class="detailHead">
				<code>{field.name || t.realms.fields.fallbackName}</code>
				<span class="declaredAs">{declaredText(field.ty)}</span>
			</span>

			{#if inner.type === 'range' || inner.type === 'span'}
				{@const bounds = inner}
				<div class="bounds">
					<label>
						<span>{t.realms.fields.from}</span>
						<NumberInput
							value={bounds.min}
							ariaLabel={t.realms.fields.lowest}
							onchange={(next) =>
								set(at, {
									ty: withList(
										{ type: bounds.type, min: Number(next) || 0, max: bounds.max },
										isList(field.ty)
									)
								})}
						/>
					</label>
					<label>
						<span>{t.realms.fields.to}</span>
						<NumberInput
							value={bounds.max}
							ariaLabel={t.realms.fields.highest}
							onchange={(next) =>
								set(at, {
									ty: withList(
										{ type: bounds.type, min: bounds.min, max: Number(next) || 0 },
										isList(field.ty)
									)
								})}
						/>
					</label>
				</div>
			{:else if inner.type === 'enum'}
				{@const choices = inner}
				<label>
					<span>{t.realms.fields.choices}</span>
					<input
						value={choices.variants.join(', ')}
						placeholder={t.realms.fields.choicesPlaceholder}
						onchange={(e) =>
							set(at, {
								ty: withList(
									{ type: 'enum', variants: splitVariants(e.currentTarget.value) },
									isList(field.ty)
								)
							})}
					/>
				</label>
			{:else if inner.type === 'object'}
				{@const group = inner}
				<Self
					fields={group.fields}
					depth={depth + 1}
					{ping}
					onchange={(next) =>
						set(at, { ty: withList({ type: 'object', fields: next }, isList(field.ty)) })}
				/>
			{:else}
				<p class="note">{t.realms.fields.nothingMore}</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.fields {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.head button {
		display: flex;
		padding: 2px;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
	}
	.head button:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.deep {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin: 0;
		color: var(--color-warning-text);
		font-size: var(--font-size-xs);
	}

	table {
		width: 100%;
		border-collapse: collapse;
		table-layout: fixed;
	}
	th {
		padding: 2px var(--spacing-xs);
		text-align: left;
		font-weight: var(--font-weight-normal);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	th:last-child {
		width: 24px;
	}
	th.checkCol {
		width: 2.5em;
		text-align: center;
	}
	td {
		padding: 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	td.checkCol {
		text-align: center;
	}
	td :global(input:not([type='checkbox'])),
	td :global(select) {
		width: 100%;
		min-width: 0;
		padding: 3px var(--spacing-xs);
		background-color: transparent;
		border: none;
		border-radius: 0;
		font-size: var(--font-size-sm);
	}
	td :global(input:not([type='checkbox']):focus),
	td :global(select:focus) {
		background-color: var(--color-bg-input);
	}
	tbody tr:hover td {
		background-color: var(--color-hover);
	}
	tbody tr.selected td {
		background-color: var(--color-selected);
	}
	.mono {
		font-family: var(--font-family-mono);
	}
	.remove {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		padding: var(--spacing-xs) 0;
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.remove:hover {
		color: var(--color-error-text);
	}

	.detail {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-top: none;
		border-radius: 0 0 var(--radius-sm) var(--radius-sm);
	}
	.detailHead {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.detailHead code {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
	}
	.declaredAs {
		flex: 1 1 auto;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: right;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.bounds {
		display: flex;
		gap: var(--spacing-sm);
	}
	.bounds label,
	.detail > label {
		display: flex;
		flex: 1 1 0;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.bounds label > span,
	.detail > label > span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
