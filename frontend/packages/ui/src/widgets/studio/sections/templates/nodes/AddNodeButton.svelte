<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	type Kind = 'text' | 'source';

	let { onpick }: { onpick: (kind: Kind) => void } = $props();

	let open = $state(false);
	let hovered = $state<Kind | null>(null);

	// gesture bookkeeping
	let dragging = false;
	let moved = false;
	let startX = 0;
	let startY = 0;
	let pointerId: number | null = null;

	let triggerEl: HTMLButtonElement;
	let rootEl: HTMLDivElement;

	const MOVE_THRESHOLD = 4;

	/** Which option sits under these client coords. */
	function optionAt(x: number, y: number): Kind | null {
		const el = document.elementFromPoint(x, y);
		const opt = el?.closest<HTMLElement>('[data-kind]');
		const k = opt?.dataset.kind;
		return k === 'text' || k === 'source' ? k : null;
	}

	function onPointerDown(e: PointerEvent) {
		if (e.button !== 0) return;
		e.preventDefault();
		open = true;
		dragging = true;
		moved = false;
		hovered = null;
		startX = e.clientX;
		startY = e.clientY;
		pointerId = e.pointerId;
		try {
			triggerEl.setPointerCapture(e.pointerId);
		} catch {
			// Stubbed for now, not sure if it's even needed
			return;
		}
	}

	function onPointerMove(e: PointerEvent) {
		if (!dragging) return;
		if (!moved && Math.hypot(e.clientX - startX, e.clientY - startY) > MOVE_THRESHOLD) {
			moved = true;
		}
		hovered = optionAt(e.clientX, e.clientY);
	}

	function releaseCapture() {
		if (pointerId !== null) {
			try {
				triggerEl.releasePointerCapture(pointerId);
			} catch {
				/* already released */
			}
			pointerId = null;
		}
	}

	function onPointerUp(e: PointerEvent) {
		if (!dragging) return;
		dragging = false;
		releaseCapture();

		const overOption = optionAt(e.clientX, e.clientY);
		const el = document.elementFromPoint(e.clientX, e.clientY);
		const overTrigger = !!el && (el === triggerEl || triggerEl.contains(el));

		if (overOption && moved) {
			pick(overOption);
		} else if (overTrigger || (overOption && !moved)) {
			open = true;
		} else {
			open = false;
		}
		hovered = null;
	}

	function onPointerCancel() {
		dragging = false;
		releaseCapture();
		hovered = null;
	}

	function pick(kind: Kind) {
		open = false;
		hovered = null;
		dragging = false;
		onpick(kind);
	}

	function onTriggerKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
			e.preventDefault();
			open = true;
		} else if (e.key === 'Escape') {
			open = false;
		}
	}

	$effect(() => {
		if (!open) return;
		const onDocDown = (ev: PointerEvent) => {
			if (dragging) return;
			const t = ev.target as Node | null;
			if (t && rootEl.contains(t)) return;
			open = false;
		};
		const onKey = (ev: KeyboardEvent) => {
			if (ev.key === 'Escape') open = false;
		};
		window.addEventListener('pointerdown', onDocDown, true);
		window.addEventListener('keydown', onKey);
		return () => {
			window.removeEventListener('pointerdown', onDocDown, true);
			window.removeEventListener('keydown', onKey);
		};
	});
</script>

<div class="add-node" bind:this={rootEl}>
	<button
		bind:this={triggerEl}
		class="add-btn trigger"
		type="button"
		aria-haspopup="menu"
		aria-expanded={open}
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointercancel={onPointerCancel}
		onkeydown={onTriggerKeydown}
	>
		<span>{t.studio.templateEditor.addNode}</span>
		<svg class="chev" viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
			<path
				d="M2 4l4 4 4-4"
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>
		</svg>
	</button>

	{#if open}
		<div class="menu" role="menu">
			<button
				class="option"
				class:active={hovered === 'text'}
				type="button"
				role="menuitem"
				data-kind="text"
				onclick={() => pick('text')}
			>
				{t.studio.templateEditor.textNode}
			</button>
			<button
				class="option"
				class:active={hovered === 'source'}
				type="button"
				role="menuitem"
				data-kind="source"
				onclick={() => pick('source')}
			>
				{t.studio.templateEditor.sourceNode}
			</button>
		</div>
	{/if}
</div>

<style>
	.add-node {
		position: relative;
		display: inline-block;
	}

	.trigger {
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		white-space: nowrap;
		touch-action: none;
	}
	.chev {
		flex: 0 0 auto;
		display: block;
		margin-left: var(--spacing-sm);
	}

	.menu {
		position: absolute;
		top: calc(100% + var(--spacing-xs));
		left: 0;
		z-index: var(--z-index-dropdown);
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 140px;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}

	.option {
		width: 100%;
		justify-content: flex-start;
		background-color: transparent;
		font-size: var(--font-size-base);
	}
	.option:hover,
	.option.active {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
</style>
