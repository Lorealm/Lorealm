<script module lang="ts">
	export interface SearchOption {
		value: string;
		label: string;
		/** Secondary line under the label — an id, a size, whatever disambiguates. */
		hint?: string;
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { ChevronDown } from '@lucide/svelte';

	let {
		value,
		options,
		onchange,
		placeholder = '',
		emptyText = 'No matches.',
		/** Accept text that matches no option. Off means free text reverts on commit. */
		allowCustom = false,
		ariaLabel
	}: {
		value: string;
		options: SearchOption[];
		onchange: (value: string) => void;
		placeholder?: string;
		emptyText?: string;
		allowCustom?: boolean;
		ariaLabel?: string;
	} = $props();

	const listId = $props.id();

	let text = $state(untrack(() => value));
	let open = $state(false);
	let typing = $state(false);
	let activeIndex = $state(-1);
	let inputEl = $state<HTMLInputElement>();
	let listEl = $state<HTMLElement>();

	$effect(() => {
		text = value;
	});

	const matches = $derived.by(() => {
		const needle = typing ? text.trim().toLowerCase() : '';
		if (!needle) return options;
		return options.filter(
			(o) =>
				o.value.toLowerCase().includes(needle) ||
				o.label.toLowerCase().includes(needle) ||
				(o.hint ?? '').toLowerCase().includes(needle)
		);
	});

	// Keep the highlight on screen while arrowing through a long list.
	$effect(() => {
		if (activeIndex < 0) return;
		listEl?.querySelector<HTMLElement>(`[data-row="${activeIndex}"]`)?.scrollIntoView({
			block: 'nearest'
		});
	});

	function openMenu() {
		open = true;
		typing = false;
		activeIndex = -1;
	}

	function close() {
		open = false;
		typing = false;
		activeIndex = -1;
	}

	function commit() {
		const trimmed = text.trim();
		if (!allowCustom && !options.some((o) => o.value === trimmed)) {
			text = value;
			return;
		}
		text = trimmed;
		if (trimmed !== value) onchange(trimmed);
	}

	function pick(option: SearchOption) {
		text = option.value;
		if (option.value !== value) onchange(option.value);
		close();
	}

	function move(delta: number) {
		if (matches.length === 0) return;
		const next = activeIndex + delta;
		activeIndex = next < 0 ? matches.length - 1 : next >= matches.length ? 0 : next;
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			if (open) move(1);
			else openMenu();
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			if (open) move(-1);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const active = open && activeIndex >= 0 ? matches[activeIndex] : undefined;
			if (active) pick(active);
			else {
				commit();
				close();
			}
		} else if (e.key === 'Escape' && open) {
			e.preventDefault();
			text = value; // abandon the edit
			close();
		}
	}

	// The field keeps focus after a pick, so a second click fires no focus event.
	function onClick() {
		if (open) return;
		open = true;
		activeIndex = -1;
	}

	function onInput() {
		typing = true;
		open = true;
		activeIndex = -1;
	}

	// Clicking a row must not blur the field: the blur would commit and tear the
	// menu down before the click ever lands.
	function holdFocus(e: MouseEvent) {
		e.preventDefault();
	}
</script>

<div class="search-select">
	<div class="control">
		<input
			type="text"
			role="combobox"
			bind:this={inputEl}
			bind:value={text}
			{placeholder}
			aria-label={ariaLabel}
			aria-expanded={open}
			aria-controls={listId}
			aria-autocomplete="list"
			aria-activedescendant={open && activeIndex >= 0 ? `${listId}-${activeIndex}` : undefined}
			autocomplete="off"
			spellcheck="false"
			oninput={onInput}
			onkeydown={onKeydown}
			onclick={onClick}
			onfocus={openMenu}
			onblur={() => {
				commit();
				close();
			}}
		/>
		<button
			type="button"
			class="toggle"
			tabindex="-1"
			aria-label={open ? 'Hide suggestions' : 'Show suggestions'}
			onmousedown={holdFocus}
			onclick={() => {
				if (open) close();
				else {
					inputEl?.focus();
					openMenu();
				}
			}}
		>
			<ChevronDown size={12} />
		</button>
	</div>

	{#if open}
		<div class="panel">
			<div class="options" id={listId} role="listbox" aria-label={ariaLabel} bind:this={listEl}>
				{#each matches as option, i (option.value)}
					<button
						type="button"
						class="option"
						class:active={activeIndex === i}
						id={`${listId}-${i}`}
						role="option"
						aria-selected={option.value === value}
						data-row={i}
						onmousemove={() => (activeIndex = i)}
						onmousedown={holdFocus}
						onclick={() => pick(option)}
					>
						<span class="text">
							<span class="label">{option.label}</span>
							{#if option.hint}<span class="hint">{option.hint}</span>{/if}
						</span>
					</button>
				{:else}
					<p class="empty">{emptyText}</p>
				{/each}
			</div>
		</div>
	{/if}
</div>

<style>
	.search-select {
		position: relative;
		display: flex;
		min-width: 0;
	}

	.control {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		background-color: var(--color-bg-elevated);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		padding-right: var(--spacing-xs);
	}
	.control:focus-within {
		border-color: var(--color-border-active);
	}
	.control input {
		flex: 1 1 auto;
		min-width: 0;
		background: transparent;
		border: none;
	}
	.toggle {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-muted);
	}

	.toggle:hover {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-accent);
	}

	.panel {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		right: 0;
		z-index: 50;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}
	.options {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 16em;
		overflow-y: auto;
	}
	.option {
		display: flex;
		align-items: center;
		width: 100%;
		padding: var(--spacing-xs) var(--spacing-sm);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-secondary);
		text-align: left;
	}
	.option.active {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.option[aria-selected='true'] {
		color: var(--color-text-primary);
	}
	.text {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--font-size-sm);
	}
	.hint {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.empty {
		margin: 0;
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
