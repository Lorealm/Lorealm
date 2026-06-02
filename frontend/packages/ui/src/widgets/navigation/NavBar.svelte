<script lang="ts">
	import {
		ChevronLeft,
		Earth,
		MenuIcon,
		Palette,
		PencilIcon,
		Settings,
		User
	} from '@lucide/svelte';
	import NavButton from './NavButton.svelte';
	import { SidebarView } from '$lib/stores/navigation.svelte';
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
			<div class="branding">Eldoria</div>
		{/if}
		<button class="collapse-button" onclick={toggleCollapse}>
			{#if collapsed}
				<MenuIcon />
			{:else}
				<ChevronLeft />
			{/if}
		</button>
	</div>
	<NavButton label="Write" Icon={PencilIcon} {collapsed} view={SidebarView.Write} />
	<NavButton label="Characters" Icon={User} {collapsed} view={SidebarView.Characters} />
	<NavButton label="Realms" Icon={Earth} {collapsed} view={SidebarView.Realm} />
	<NavButton label="Studio" Icon={Palette} {collapsed} view={SidebarView.Studio} />

	<div style="margin-top: auto; margin-bottom: 0.33em;">
		<NavButton label="Settings" Icon={Settings} {collapsed} view={SidebarView.Settings} />
	</div>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--width-sidebar-full);
		background-color: var(--color-bg-elevated);
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
	}
	.collapse-button {
		flex-shrink: 1;
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
