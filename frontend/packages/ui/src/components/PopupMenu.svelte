<script lang="ts">
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import type { Component, Snippet } from 'svelte';

	type MenuItem = {
		label: string;
		icon: Component | undefined;
		onSelect: () => void;
		disabled?: boolean;
	};

	let {
		items,
		title,
		icon,
		placement = 'up'
	}: {
		items: MenuItem[];
		title?: string;
		icon: Snippet;
		placement?: 'up' | 'down';
	} = $props();

	let open = $state(false);
	let activeIndex = $state(-1);

	let triggerEl: HTMLButtonElement;
	let menuEl = $state<HTMLElement>();

	// true while the press that opened the menu is still held (drag-select capable)
	let pressing = false;
	// did the pointer leave the trigger during that press?
	let movedAway = false;

	function itemIndexAtPoint(x: number, y: number) {
		const el = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-item]');
		return el ? Number(el.dataset.item) : -1;
	}

	function overTrigger(x: number, y: number) {
		return triggerEl.contains(document.elementFromPoint(x, y));
	}

	function close() {
		open = false;
		activeIndex = -1;
		pressing = false;
		movedAway = false;
	}

	function onPointerDown(e: PointerEvent) {
		if (e.button !== 0) return;
		e.preventDefault();
		triggerEl.focus();

		if (open) {
			close(); // second press toggles closed
			return;
		}
		open = true;
		activeIndex = -1;
		pressing = true;
		movedAway = false;
		triggerEl.setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!pressing) return;
		if (!overTrigger(e.clientX, e.clientY)) movedAway = true;
		activeIndex = itemIndexAtPoint(e.clientX, e.clientY);
	}

	function onPointerUp(e: PointerEvent) {
		if (triggerEl.hasPointerCapture(e.pointerId)) triggerEl.releasePointerCapture(e.pointerId);
		if (!pressing) return;

		const idx = itemIndexAtPoint(e.clientX, e.clientY);
		if (idx >= 0) {
			// released on an item → count it as a click
			if (!items[idx].disabled) {
				items[idx].onSelect();
				close();
			}
			return; // disabled item: leave menu open
		}

		// quick press+release on the trigger → leave menu open (normal click mode)
		if (overTrigger(e.clientX, e.clientY) && !movedAway) {
			pressing = false;
			return;
		}

		close();
	}

	function select(item: MenuItem) {
		if (item.disabled) return;
		item.onSelect();
		close();
	}

	function onWindowPointerDown(e: PointerEvent) {
		if (!open || pressing) return;
		const t = e.target as Node;
		if (triggerEl.contains(t) || menuEl?.contains(t)) return;
		close();
	}

	function onWindowKeyDown(e: KeyboardEvent) {
		if (open && e.key === 'Escape') close();
	}
</script>

<svelte:window onpointerdown={onWindowPointerDown} onkeydown={onWindowKeyDown} />

<span class="popup">
	<button
		class="trigger"
		bind:this={triggerEl}
		{@attach tooltip(title)}
		aria-haspopup="menu"
		aria-expanded={open}
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointercancel={close}
	>
		{@render icon()}
	</button>

	{#if open}
		<div class="menu" class:down={placement === 'down'} role="menu" bind:this={menuEl}>
			{#each items as item, i (i)}
				<button
					class="item"
					class:active={i === activeIndex}
					role="menuitem"
					data-item={i}
					disabled={item.disabled}
					onclick={() => select(item)}
				>
					{#if item.icon}<div class="list-icon"><item.icon /></div>{/if}{item.label}
				</button>
			{/each}
		</div>
	{/if}
</span>

<style>
	.popup {
		position: relative;
		display: inline-flex;
	}
	.trigger {
		display: inline-flex;
		justify-content: center;
		align-items: center;
		touch-action: none; /* stop touch-drag from scrolling */
	}
	.trigger:focus-visible,
	.item:focus-visible {
		outline: none;
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.trigger:focus:not(:focus-visible),
	.item:focus:not(:focus-visible) {
		outline: none;
	}
	.menu {
		position: absolute;
		bottom: calc(100% + 4px);
		left: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		min-width: 10em;
		padding: var(--spacing-xs, 4px);
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
		user-select: none;
		gap: var(--spacing-xs);
		-moz-user-select: none;
	}
	.menu.down {
		top: calc(100% + 4px);
		bottom: auto;
		left: auto;
		right: 0;
	}
	.item {
		display: flex;
		align-items: center;
		width: 100%;
		padding: var(--spacing-sm) var(--spacing-sm);
		border: none;
		border-radius: var(--radius-sm, 6px);
		background: transparent;
		color: var(--color-text-primary);
		font-size: var(--font-size-base, 0.9rem);
		text-align: left;
		white-space: nowrap;
		cursor: pointer;
	}
	.item:hover,
	.item.active {
		background-color: var(--color-bg-base);
	}
	.item:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.list-icon {
		display: flex;
		margin-right: var(--spacing-sm);
		height: 0.9em;
		justify-content: center;
		align-items: center;
	}
</style>
