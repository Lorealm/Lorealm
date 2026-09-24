<script lang="ts">
	import {
		ChevronLeft,
		Earth,
		MenuIcon,
		Palette,
		PencilIcon,
		Plug2,
		Blocks,
		Settings,
		User
	} from '@lucide/svelte';
	import NavButton from './NavButton.svelte';
	import { navigator } from '$lib/navigation/navigator.js';
	var collapsed = $state(false);
	var sidebarElement: HTMLDivElement;

	function toggleCollapse() {
		collapsed = !collapsed;
		var width = collapsed ? 'var(--width-sidebar-collapsed)' : 'var(--width-sidebar-full)';
		sidebarElement.style.width = width;
	}
</script>

<div class="container" bind:this={sidebarElement}>
	<div class="top-container">
		{#if !collapsed}
			<div class="branding">Lorealm</div>
		{/if}
		<button class="collapse-button" onclick={toggleCollapse}>
			{#if collapsed}
				<MenuIcon />
			{:else}
				<ChevronLeft />
			{/if}
		</button>
	</div>
	<NavButton
		label="Write"
		Icon={PencilIcon}
		{collapsed}
		view="write"
		onselect={() => navigator.start().write()}
	/>
	<NavButton
		label="Characters"
		Icon={User}
		{collapsed}
		view="characters"
		onselect={() => navigator.start().characters()}
	/>
	<NavButton
		label="Realms"
		Icon={Earth}
		{collapsed}
		view="realms"
		onselect={() => navigator.start().realms()}
	/>
	<NavButton
		label="Connections"
		Icon={Plug2}
		{collapsed}
		view="connections"
		onselect={() => navigator.start().connections()}
	/>
	<NavButton
		label="Studio"
		Icon={Palette}
		{collapsed}
		view="studio"
		onselect={() => navigator.start().studio()}
	/>

	<div style="margin-top: auto; margin-bottom: 0.33em;">
		<NavButton
			label="Plugins"
			Icon={Blocks}
			{collapsed}
			view="plugins"
			onselect={() => navigator.start().plugins()}
		/>
		<NavButton
			label="Settings"
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
