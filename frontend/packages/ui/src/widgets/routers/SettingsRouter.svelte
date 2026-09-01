<script lang="ts">
	let { view }: { view: string } = $props();
</script>

<div class="container">
	{#if view === 'roles'}
		Roles view
	{/if}
</div>

<style>
	.container {
		display: flex;
		flex: 1;
	}
</style>
