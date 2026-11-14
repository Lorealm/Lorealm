<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import {
		ChevronLeft,
		Earth,
		MenuIcon,
		Palette,
		PencilIcon,
		Plug2,
		Blocks,
		Settings,
		User,
		X
	} from '@lucide/svelte';
	import NavButton from './NavButton.svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { shell } from '#lib/stores/shell.svelte.js';

	let userCollapsed = $state(false);
	const collapsed = $derived(userCollapsed && !shell.narrow);
</script>

<div
	class="container"
	class:collapsed
	class:open={shell.navOpen}
	inert={shell.narrow && !shell.navOpen}
>
	<div class="top-container">
		{#if !collapsed}
			<div class="branding">Lorealm</div>
		{/if}
		{#if shell.narrow}
			<button
				class="collapse-button"
				aria-label={t.nav.closeMenu}
				onclick={() => shell.closeDrawer()}
			>
				<X />
			</button>
		{:else}
			<button
				class="collapse-button"
				aria-label={collapsed ? t.nav.expandMenu : t.nav.collapseMenu}
				onclick={() => (userCollapsed = !userCollapsed)}
			>
				{#if collapsed}
					<MenuIcon />
				{:else}
					<ChevronLeft />
				{/if}
			</button>
		{/if}
	</div>
	<NavButton
		label={t.nav.views.write}
		Icon={PencilIcon}
		{collapsed}
		view="write"
		onselect={() => navigator.start().write()}
	/>
	<NavButton
		label={t.nav.views.characters}
		Icon={User}
		{collapsed}
		view="characters"
		onselect={() => navigator.start().characters()}
	/>
	<NavButton
		label={t.nav.views.realms}
		Icon={Earth}
		{collapsed}
		view="realms"
		onselect={() => navigator.start().realms()}
	/>
	<NavButton
		label={t.nav.views.connections}
		Icon={Plug2}
		{collapsed}
		view="connections"
		onselect={() => navigator.start().connections()}
	/>
	<NavButton
		label={t.nav.views.studio}
		Icon={Palette}
		{collapsed}
		view="studio"
		onselect={() => navigator.start().studio()}
	/>

	<div style="margin-top: auto; margin-bottom: 0.33em;">
		<NavButton
			label={t.nav.views.plugins}
			Icon={Blocks}
			{collapsed}
			view="plugins"
			onselect={() => navigator.start().plugins()}
		/>
		<NavButton
			label={t.nav.views.settings}
			Icon={Settings}
			{collapsed}
			view="settings"
			onselect={() => navigator.start().settings()}
		/>
	</div>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--width-sidebar-full);
		background-color: var(--color-bg-elevated);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
		border-right: 1px solid var(--color-border-subtle);
	}
	.container.collapsed {
		width: var(--width-sidebar-collapsed);
	}
	@media (max-width: 768px) {
		.container {
			position: fixed;
			inset: 0 auto 0 0;
			z-index: var(--z-index-always-top);
			width: min(var(--width-sidebar-full), 85vw);
			background-color: var(--color-bg-panel-solid);
			transform: translateX(-100%);
			visibility: hidden;
			transition:
				transform var(--transition-fast) var(--easing-default),
				visibility 0s var(--transition-fast);
		}
		.container.open {
			transform: none;
			visibility: visible;
			transition: transform var(--transition-fast) var(--easing-default);
			box-shadow: var(--shadow-xl);
		}
	}
	@media (max-width: 768px) and (prefers-reduced-motion: reduce) {
		.container,
		.container.open {
			transition: none;
		}
	}
	.top-container {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		height: var(--height-header);
		margin-bottom: 0.5em;
		border-bottom: 1px solid var(--color-border-subtle);
		background: var(--gradient-bg);
	}
	.collapse-button {
		flex-shrink: 1;
		background-color: transparent;
	}
	.branding {
		display: flex;
		font-size: var(--font-size-h2);
		font-family: var(--font-family-quicksand);
		font-weight: var(--font-weight-medium);
		color: var(--color-text-accent);
		margin-left: var(--spacing-sm);
		user-select: none;
		-webkit-user-select: none;
	}
</style>
