<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { CircleDot, X } from '@lucide/svelte';
	import type { RealmResourceKind } from '#lib/types/realm.js';
	import { activeDrag, carriesAsset, clearDrag, readDrag, type AssetDrag } from './drag.svelte.js';
	import {
		REF_NAME,
		bindingLabel,
		isList,
		loopReaches,
		referencePaths,
		type Binding,
		type InputDecl
	} from './model.js';
	import { ensureLoaded, itemsOf, RESOURCE_KINDS, type ResourceItem } from './resources.svelte.js';

	/** A loop variable an enclosing `foreach` puts in scope. */
	export interface LoopVar {
		varName: string;
		input: InputDecl | null;
	}

	let {
		value,
		onchange,
		/** Which resource kinds may be picked. Omitted means all of them. */
		kinds = RESOURCE_KINDS,
		/** Loop variables in scope here, from `realmDraft.scopeOf`. */
		scope = [],
		/** The document's input declarations; singular entity ones bind directly. */
		inputs = [],
		/** Jump to whatever the field points at. Inspired by how Unity does it. */
		onlocate,
		placeholder,
		ariaLabel
	}: {
		value: Binding | null;
		onchange: (next: Binding | null) => void;
		kinds?: RealmResourceKind[];
		scope?: LoopVar[];
		inputs?: InputDecl[];
		onlocate?: (binding: Binding, mode: 'select' | 'open') => void;
		placeholder?: string;
		ariaLabel?: string;
	} = $props();

	const listId = $props.id();

	let open = $state(false);
	let query = $state('');
	let activeIndex = $state(0);
	let dropping = $state(false);
	let rootEl = $state<HTMLElement>();
	let inputEl = $state<HTMLInputElement>();
	let listEl = $state<HTMLElement>();

	$effect(() => {
		if (open) ensureLoaded(kinds);
	});

	interface Row {
		row: 'loop' | 'input' | 'resource';
		key: string;
		group: string;
		label: string;
		hint: string;
		binding: Binding;
	}

	const loopRows = $derived.by(() => {
		const rows: Row[] = [];
		for (const { varName, input } of scope) {
			if (!input) continue;
			for (const { path, kind } of loopReaches(input.ty)) {
				if (!kind || !kinds.includes(kind)) continue;
				const reach = [varName, ...path, 'id'].join('.');
				rows.push({
					row: 'loop',
					key: `loop:${reach}`,
					group: t.realms.binding.fromLoop,
					label: reach,
					hint: t.realms.binding.each({ type: REF_NAME[kind], input: input.name }),
					binding: { source: 'loop', varName, path }
				});
			}
		}
		return rows;
	});

	const inputRows = $derived.by(() =>
		inputs
			.filter((decl) => !isList(decl.ty))
			.flatMap((decl) =>
				referencePaths(decl.ty)
					.filter(({ kind }) => kind !== null && kinds.includes(kind))
					.map(({ path }): Row => {
						const reach = [`input:${decl.name}`, ...path, 'id'].join('.');
						return {
							row: 'input',
							key: reach,
							group: t.realms.binding.fromInput,
							label: reach,
							hint: t.realms.binding.chosenOnExpand,
							binding: { source: 'input', name: decl.name, path }
						};
					})
			)
	);

	const resourceRows = $derived.by(() =>
		kinds.flatMap((kind) =>
			itemsOf(kind).map((item: ResourceItem): Row => ({
				row: 'resource',
				key: `${kind}:${item.id}`,
				group: REF_NAME[kind],
				label: item.name,
				hint: item.hint ?? '',
				binding: { source: 'resource', kind, id: item.id, name: item.name }
			}))
		)
	);

	const rows = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const all = [...loopRows, ...inputRows, ...resourceRows];
		if (!needle) return all;
		return all.filter(
			(r) =>
				r.label.toLowerCase().includes(needle) ||
				r.hint.toLowerCase().includes(needle) ||
				r.group.toLowerCase().includes(needle)
		);
	});

	$effect(() => {
		if (activeIndex >= rows.length) activeIndex = Math.max(0, rows.length - 1);
	});

	$effect(() => {
		if (!open) return;
		listEl?.querySelector<HTMLElement>(`[data-row="${activeIndex}"]`)?.scrollIntoView({
			block: 'nearest'
		});
	});

	const label = $derived(
		value === null ? (placeholder ?? t.realms.binding.none) : bindingLabel(value)
	);

	function openPicker() {
		open = true;
		query = '';
		activeIndex = 0;
		queueMicrotask(() => inputEl?.focus());
	}

	function close() {
		open = false;
	}

	function pick(binding: Binding) {
		onchange(binding);
		close();
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			if (rows.length === 0) return;
			const next = activeIndex + (event.key === 'ArrowDown' ? 1 : -1);
			activeIndex = next < 0 ? rows.length - 1 : next >= rows.length ? 0 : next;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			const row = rows[activeIndex];
			if (row) pick(row.binding);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			close();
		}
	}

	function holdFocus(event: MouseEvent) {
		event.preventDefault();
	}

	/**
	 * A dropped input binds one of two ways. A singular entity input reads
	 * straight off `input:`; a list only reads through a loop that walks it.
	 */
	function bindingFor(payload: AssetDrag): Binding | null {
		if (payload.drag === 'resource') {
			return kinds.includes(payload.kind)
				? { source: 'resource', kind: payload.kind, id: payload.id, name: payload.name }
				: null;
		}
		const decl = inputs.find((i) => i.name === payload.name);
		if (decl && !isList(decl.ty)) {
			const usable = referencePaths(decl.ty).filter(
				({ kind }) => kind !== null && kinds.includes(kind)
			);
			const dragged = payload.path.join('.');
			const reach =
				usable.find((r) => r.path.join('.') === dragged) ?? (dragged ? undefined : usable[0]);
			return reach ? { source: 'input', name: decl.name, path: reach.path } : null;
		}
		const inScope = scope.find((v) => v.input?.name === payload.name);
		if (!inScope?.input) return null;
		const reach = loopReaches(inScope.input.ty).find(
			({ kind }) => kind !== null && kinds.includes(kind)
		);
		return reach ? { source: 'loop', varName: inScope.varName, path: reach.path } : null;
	}

	const incoming = $derived(activeDrag());
	const compatible = $derived(incoming !== null && bindingFor(incoming) !== null);

	function onDrop(event: DragEvent) {
		dropping = false;
		clearDrag();
		const payload = readDrag(event);
		if (!payload) return;
		event.preventDefault();
		const binding = bindingFor(payload);
		if (binding) onchange(binding);
	}
</script>

<svelte:window
	onclick={(e) => {
		if (open && !rootEl?.contains(e.target as Node)) close();
	}}
/>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={rootEl}
	class="radio"
	class:dropping
	class:candidate={incoming !== null && compatible}
	class:incompatible={incoming !== null && !compatible}
	ondragover={(e) => {
		if (!carriesAsset(e) || !compatible) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
		dropping = true;
	}}
	ondragleave={() => (dropping = false)}
	ondrop={onDrop}
>
	<div class="field">
		<button
			type="button"
			class="content"
			class:empty={value === null}
			class:loop={value !== null && value.source !== 'resource'}
			aria-label={value === null
				? t.realms.binding.emptyAria({ label: ariaLabel ?? t.realms.binding.field })
				: t.realms.binding.filledAria({ label: ariaLabel ?? t.realms.binding.field, value: label })}
			onclick={() => {
				if (value === null) return open ? close() : openPicker();
				onlocate?.(value, 'select');
			}}
			ondblclick={() => value !== null && onlocate?.(value, 'open')}
		>
			<span class="label">{label}</span>
		</button>

		{#if value !== null}
			<button
				type="button"
				class="clear"
				{@attach tooltip(t.realms.binding.clear)}
				aria-label={t.realms.binding.clearAria({ label: ariaLabel ?? t.realms.binding.fieldLower })}
				onclick={() => onchange(null)}
			>
				<X size={12} />
			</button>
		{/if}

		<button
			type="button"
			class="pick"
			{@attach tooltip(t.realms.binding.pick)}
			aria-label={t.realms.binding.pickAria({ label: ariaLabel ?? t.realms.binding.thisField })}
			role="combobox"
			aria-expanded={open}
			aria-controls={listId}
			aria-haspopup="listbox"
			onclick={() => (open ? close() : openPicker())}
		>
			<CircleDot size={14} />
		</button>
	</div>

	{#if open}
		<div class="panel">
			<input
				bind:this={inputEl}
				bind:value={query}
				type="text"
				class="search"
				placeholder={t.realms.binding.search}
				autocomplete="off"
				spellcheck="false"
				aria-controls={listId}
				aria-activedescendant={rows.length ? `${listId}-${activeIndex}` : undefined}
				onkeydown={onKeydown}
			/>
			<div class="rows" id={listId} role="listbox" aria-label={ariaLabel} bind:this={listEl}>
				{#each rows as row, i (row.key)}
					{#if i === 0 || rows[i - 1].group !== row.group}
						<div class="group">{row.group}</div>
					{/if}
					<button
						type="button"
						class="option"
						class:active={i === activeIndex}
						class:loop={row.row !== 'resource'}
						id={`${listId}-${i}`}
						role="option"
						aria-selected={i === activeIndex}
						data-row={i}
						onmousemove={() => (activeIndex = i)}
						onmousedown={holdFocus}
						onclick={() => pick(row.binding)}
					>
						<span class="name">{row.label}</span>
						{#if row.hint}<span class="hint">{row.hint}</span>{/if}
					</button>
				{:else}
					<p class="empty-list">{t.realms.binding.nothing}</p>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.radio {
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.field {
		display: flex;
		align-items: stretch;
		width: 100%;
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
	}
	.field:hover,
	.field:focus-within {
		background-color: var(--color-bg-elevated);
		border-color: var(--color-border-active);
		color: var(--color-text-primary);
	}
	.radio.candidate .field {
		border-color: var(--color-border-active);
		border-style: dashed;
	}
	.radio.incompatible {
		opacity: 0.35;
		filter: grayscale(1);
	}
	.radio.dropping .field {
		border-color: var(--color-primary-light);
		border-style: solid;
		box-shadow: var(--shadow-glow);
	}

	.content {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		align-items: center;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border: none;
		color: inherit;
		text-align: left;
	}
	.content.empty {
		color: var(--color-text-muted);
		font-style: italic;
	}
	.content.loop {
		font-family: var(--font-family-mono);
		color: var(--color-text-accent);
		font-style: normal;
	}
	.label {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
	}

	.clear {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		padding: 0 var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		opacity: 0;
	}
	.field:hover .clear,
	.clear:focus-visible {
		opacity: 1;
	}
	.clear:hover {
		color: var(--color-error-text);
	}

	.pick {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		padding: 0 var(--spacing-sm);
		background: transparent;
		border: none;
		border-left: 1px solid var(--color-border-strong);
		color: inherit;
	}
	.pick:hover {
		color: var(--color-text-accent);
	}

	.panel {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		right: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		min-width: 220px;
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}
	.search {
		width: 100%;
		font-size: var(--font-size-sm);
	}
	.rows {
		display: flex;
		flex-direction: column;
		gap: 1px;
		max-height: 18em;
		overflow-y: auto;
	}
	.group {
		padding: var(--spacing-xs) var(--spacing-sm) 2px;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	.option {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		width: 100%;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		text-align: left;
	}
	.option.active {
		background-color: var(--color-hover);
		color: var(--color-text-primary);
	}
	.option.loop .name {
		font-family: var(--font-family-mono);
		color: var(--color-text-accent);
	}
	.name {
		flex: 0 1 auto;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
	}
	.hint {
		flex: 1 1 auto;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: right;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.empty-list {
		margin: 0;
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	@media (hover: none) {
		.clear {
			opacity: 1;
		}
	}
</style>
