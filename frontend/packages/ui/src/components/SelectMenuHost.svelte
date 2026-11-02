<!--
	Themes every native <select> in the app without call sites opting in. Browsers
	won't style an open dropdown, so this takes over opening one: the list comes
	from the select's real <option>s, and a pick goes back through the select with
	a real change event. `bind:value`, non-string values and `onchange` all keep
	working, and a <select> added later is covered automatically.
-->
<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Check } from '@lucide/svelte';
	import { placeFloating } from './floating.ts';

	type Row =
		| { kind: 'heading'; label: string }
		| { kind: 'divider' }
		| { kind: 'option'; index: number; label: string; disabled: boolean; nested: boolean };

	const TYPEAHEAD_RESET = 500;
	const OPEN_KEYS = new Set([' ', 'Enter', 'ArrowDown', 'ArrowUp']);

	let select = $state<HTMLSelectElement | null>(null);
	let rows = $state<Row[]>([]);
	let selected = $state(-1);
	let active = $state(-1);
	let panel = $state<HTMLElement>();

	let typed = '';
	let typedAt = 0;

	function isThemed(el: EventTarget | null): el is HTMLSelectElement {
		return el instanceof HTMLSelectElement && !el.disabled && !el.multiple && el.size <= 1;
	}

	function readRows(el: HTMLSelectElement): Row[] {
		const out: Row[] = [];
		const addOption = (option: HTMLOptionElement, nested: boolean) => {
			if (option.hidden) return;
			out.push({
				kind: 'option',
				index: option.index,
				label: option.label,
				disabled: option.disabled,
				nested
			});
		};
		for (const child of el.children) {
			if (child instanceof HTMLOptionElement) addOption(child, false);
			else if (child instanceof HTMLHRElement) out.push({ kind: 'divider' });
			else if (child instanceof HTMLOptGroupElement) {
				out.push({ kind: 'heading', label: child.label });
				for (const option of child.children) {
					if (option instanceof HTMLOptionElement) addOption(option, true);
				}
			}
		}
		return out;
	}

	const enabled = $derived(
		rows.flatMap((row) => (row.kind === 'option' && !row.disabled ? [row] : []))
	);

	async function open(el: HTMLSelectElement) {
		rows = readRows(el);
		selected = el.selectedIndex;
		active = selected;
		select = el;
		el.setAttribute('aria-expanded', 'true');
		typed = '';
		await tick();
		if (select !== el || !panel) return;
		// The panel is at least as wide as the control it drops from.
		const anchor = el.getBoundingClientRect();
		panel.style.minWidth = `${anchor.width}px`;
		placeFloating(panel, anchor);
		panel.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
	}

	function close() {
		select?.removeAttribute('aria-expanded');
		select = null;
		active = -1;
	}

	function pick(index: number) {
		const el = select;
		close();
		if (!el?.isConnected || index === el.selectedIndex) return;
		el.selectedIndex = index;
		el.dispatchEvent(new Event('input', { bubbles: true }));
		el.dispatchEvent(new Event('change', { bubbles: true }));
	}

	function activate(index: number) {
		active = index;
		panel?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: 'nearest' });
	}

	function step(delta: number) {
		const position = enabled.findIndex((row) => row.index === active);
		const next = position < 0 && delta < 0 ? enabled.length - 1 : position + delta;
		const row = enabled[Math.max(0, Math.min(next, enabled.length - 1))];
		if (row) activate(row.index);
	}

	function typeahead(key: string) {
		const now = performance.now();
		typed = now - typedAt > TYPEAHEAD_RESET ? key : typed + key;
		typedAt = now;
		const needle = typed.toLowerCase();
		const match = enabled.find((row) => row.label.toLowerCase().startsWith(needle));
		if (match) activate(match.index);
	}

	function onOpenKey(e: KeyboardEvent) {
		switch (e.key) {
			case 'Escape':
			case 'Tab':
				close();
				if (e.key === 'Tab') return; // focus still moves on, as it does natively
				break;
			case 'Enter':
				if (active >= 0) pick(active);
				else close();
				break;
			case 'ArrowDown':
				step(1);
				break;
			case 'ArrowUp':
				step(-1);
				break;
			case 'Home':
				if (enabled[0]) activate(enabled[0].index);
				break;
			case 'End':
				if (enabled.length) activate(enabled[enabled.length - 1].index);
				break;
			case ' ':
				if (typed && performance.now() - typedAt < TYPEAHEAD_RESET) typeahead(' ');
				else if (active >= 0) pick(active);
				break;
			default:
				if (e.key.length !== 1 || e.ctrlKey || e.metaKey || e.altKey) return;
				typeahead(e.key);
		}
		// Keeps this keystroke from also closing the modal or firing a keybind.
		e.preventDefault();
		e.stopImmediatePropagation();
	}

	function onKeyDown(e: KeyboardEvent) {
		if (select) return onOpenKey(e);
		if (e.ctrlKey || e.metaKey || !OPEN_KEYS.has(e.key) || !isThemed(e.target)) return;
		e.preventDefault();
		e.stopImmediatePropagation();
		open(e.target);
	}

	function onMouseDown(e: MouseEvent) {
		if (panel?.contains(e.target as Node)) return;
		const wasOpen = select;
		close();
		if (e.button !== 0 || !isThemed(e.target)) return;
		e.preventDefault(); // stops the native popup — and focus, so that's done by hand
		e.target.focus();
		if (e.target !== wasOpen) open(e.target);
	}

	function onScroll(e: Event) {
		if (select && !panel?.contains(e.target as Node)) close();
	}

	onMount(() => {
		// Phones have a far better picker of their own than a tiny list.
		const coarse = window.matchMedia('(pointer: coarse)').matches;
		if (coarse) return;
		window.addEventListener('mousedown', onMouseDown, true);
		window.addEventListener('keydown', onKeyDown, true);
		window.addEventListener('scroll', onScroll, true);
		window.addEventListener('resize', close);
		window.addEventListener('blur', close);
		return () => {
			close();
			window.removeEventListener('mousedown', onMouseDown, true);
			window.removeEventListener('keydown', onKeyDown, true);
			window.removeEventListener('scroll', onScroll, true);
			window.removeEventListener('resize', close);
			window.removeEventListener('blur', close);
		};
	});
</script>

{#if select}
	<div class="select-menu" role="listbox" bind:this={panel}>
		{#each rows as row, i (i)}
			{#if row.kind === 'heading'}
				<div class="heading" role="presentation">{row.label}</div>
			{:else if row.kind === 'divider'}
				<hr />
			{:else}
				<div
					class="option"
					class:active={row.index === active}
					class:nested={row.nested}
					class:disabled={row.disabled}
					role="option"
					tabindex="-1"
					aria-selected={row.index === selected}
					aria-disabled={row.disabled}
					data-index={row.index}
					onmousemove={() => {
						if (!row.disabled) active = row.index;
					}}
					onmousedown={(e) => e.preventDefault()}
					onmouseup={() => {
						if (!row.disabled) pick(row.index);
					}}
				>
					<span class="label">{row.label}</span>
					{#if row.index === selected}<Check size={12} aria-hidden="true" />{/if}
				</div>
			{/if}
		{/each}
	</div>
{/if}

<style>
	.select-menu {
		position: fixed;
		top: 0;
		left: 0;
		z-index: var(--z-index-popover);
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: 2px;
		width: max-content;
		max-width: calc(100vw - 16px);
		max-height: 20rem;
		padding: var(--spacing-xs);
		overflow-y: auto;
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-xl);
		color: var(--color-text-secondary);
		font-family: var(--font-family-body);
		font-size: var(--font-size-base);
		user-select: none;
		-webkit-user-select: none;
	}
	.option {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		border-radius: var(--radius-sm);
		cursor: pointer;
	}
	.option.nested {
		padding-left: var(--spacing-lg);
	}
	.option.active {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.option[aria-selected='true'] {
		color: var(--color-text-primary);
	}
	.option.disabled {
		color: var(--color-text-disabled);
		cursor: default;
	}
	.label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.heading {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm) 2px;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-weight: var(--font-weight-semibold);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	hr {
		flex: 0 0 auto;
		width: 100%;
		margin: var(--spacing-xs) 0;
		border: none;
		border-top: 1px solid var(--color-border-subtle);
	}
</style>
