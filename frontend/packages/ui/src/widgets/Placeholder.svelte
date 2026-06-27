<script lang="ts">
	import { Construction } from '@lucide/svelte';

	let { label }: { label: string } = $props();
</script>

<div class="container">
	<div class="middle">
		<Construction size="5em" />
		<div class="display: flex; margin-top: 1em;"><b>{label}</b>: Under Construction</div>
	</div>
</div>

<style>
	.container {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
	}
	.middle {
		display: flex;
		flex-direction: column;
		color: var(--color-text-accent);
		font-size: var(--font-size-h4);
		justify-content: center;
		align-items: center;
	}
</style>
