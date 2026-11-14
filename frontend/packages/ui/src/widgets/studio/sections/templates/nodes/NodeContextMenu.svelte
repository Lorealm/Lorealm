<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import type { ContextNode } from '#lib/types/node.js';

	type ChildKind = 'text' | 'source' | 'group';

	let {
		node,
		x,
		y,
		onclose,
		onedit,
		ontoggle,
		ondelete,
		onaddchild
	}: {
		node: ContextNode;
		x: number;
		y: number;
		onclose: () => void;
		onedit: (node: ContextNode) => void;
		ontoggle: (node: ContextNode) => void;
		ondelete: (node: ContextNode) => void;
		onaddchild: (node: ContextNode, kind: ChildKind) => void;
	} = $props();

	const isGroup = $derived(node.content.type === 'group');

	let menuEl: HTMLDivElement;
	let pos = $state<{ x: number; y: number }>({ x: 0, y: 0 });
	let confirmingDelete = $state(false);
	let submenuOpen = $state(false);

	// Clamp into the viewport once we know the menu's size.
	$effect(() => {
		if (!menuEl) return;
		const r = menuEl.getBoundingClientRect();
		const nx = x + r.width > window.innerWidth ? Math.max(0, window.innerWidth - r.width - 4) : x;
		const ny =
			y + r.height > window.innerHeight ? Math.max(0, window.innerHeight - r.height - 4) : y;
		pos = { x: nx, y: ny };
	});

	$effect(() => {
		const onDown = (e: PointerEvent) => {
			const t = e.target as Node | null;
			if (t && menuEl.contains(t)) return;
			onclose();
		};
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') onclose();
		};
		window.addEventListener('pointerdown', onDown, true);
		window.addEventListener('keydown', onKey);
		return () => {
			window.removeEventListener('pointerdown', onDown, true);
			window.removeEventListener('keydown', onKey);
		};
	});

	function edit() {
		onedit(node);
		onclose();
	}
	function toggle() {
		ontoggle(node);
		onclose();
	}
	function addChild(kind: ChildKind) {
		onaddchild(node, kind);
		onclose();
	}
	function deleteClick() {
		if (!confirmingDelete) {
			confirmingDelete = true;
			return;
		}
		ondelete(node);
		onclose();
	}
</script>

<div
	bind:this={menuEl}
	class="menu"
	role="menu"
	style="left: {pos.x}px; top: {pos.y}px;"
	onmouseleave={() => (submenuOpen = false)}
	tabindex="0"
>
	<button class="item" type="button" role="menuitem" onclick={edit}
		>{t.studio.templateEditor.editMenu}</button
	>
	<button class="item" type="button" role="menuitem" onclick={toggle}>
		{node.enabled ? t.studio.templateEditor.disable : t.studio.templateEditor.enable}
	</button>

	{#if isGroup}
		<div
			class="item has-sub"
			role="menuitem"
			tabindex="0"
			onmouseenter={() => (submenuOpen = true)}
			onclick={() => (submenuOpen = !submenuOpen)}
			onkeydown={(e) => {
				if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowRight') {
					e.preventDefault();
					submenuOpen = true;
				}
			}}
		>
			<span>{t.studio.templateEditor.addChild}</span>
			<span class="chev" aria-hidden="true">▸</span>

			{#if submenuOpen}
				<div class="submenu" role="menu">
					<button class="item" type="button" role="menuitem" onclick={() => addChild('text')}>
						{t.studio.templateEditor.textNode}
					</button>
					<button class="item" type="button" role="menuitem" onclick={() => addChild('source')}>
						{t.studio.templateEditor.sourceNode}
					</button>
					<button class="item" type="button" role="menuitem" onclick={() => addChild('group')}>
						{t.studio.templateEditor.group}
					</button>
				</div>
			{/if}
		</div>
	{/if}

	<div class="sep" role="separator"></div>

	<button
		class="item danger"
		class:armed={confirmingDelete}
		type="button"
		role="menuitem"
		onclick={deleteClick}
	>
		{confirmingDelete ? t.studio.templateEditor.confirmDelete : t.common.delete}
	</button>
</div>

<style>
	.menu {
		position: fixed;
		z-index: var(--z-index-dropdown);
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 168px;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		width: 100%;
		padding: var(--spacing-sm);
		background-color: transparent;
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
		border-radius: var(--radius-sm);
		text-align: left;
		cursor: pointer;
		user-select: none;
	}
	.item:hover,
	.item:focus-visible {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
		outline: none;
	}
	.has-sub {
		position: relative;
	}
	.chev {
		color: var(--color-text-muted);
		font-size: 10px;
	}
	.submenu {
		position: absolute;
		top: -1px;
		left: 100%;
		margin-left: var(--spacing-xs);
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 150px;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
	}
	.sep {
		height: 1px;
		margin: var(--spacing-xs) 0;
		background-color: var(--color-border-subtle);
	}
	.danger {
		color: var(--color-error-text);
	}
	.danger:hover,
	.danger:focus-visible {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.danger.armed {
		background-color: var(--color-error-bg);
		border: 1px solid var(--color-error-border);
	}
</style>
