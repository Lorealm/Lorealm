<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	let { view }: { view: string } = $props();
</script>

<div class="container">
	{#if view === 'roles'}
		{t.nav.rolesPlaceholder}
	{/if}
</div>

<style>
	.container {
		display: flex;
		flex: 1;
	}
</style>
