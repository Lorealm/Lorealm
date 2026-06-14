<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<div class="widget sign-medieval">
	<span class="title">{str(props.title)}</span>
	{#if props.subtitle}
		<span class="subtitle">{str(props.subtitle)}</span>
	{/if}
</div>

<style>
	.sign-medieval {
		--wood: #5b3a21;
		--wood-dark: #3a2413;
		--carve: #d8b885;
		width: fit-content;
		max-width: 75%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-md) var(--spacing-xl);
		text-align: center;
		background:
			radial-gradient(circle at 10px 10px, #1f130a 2px, transparent 3px),
			radial-gradient(circle at calc(100% - 10px) 10px, #1f130a 2px, transparent 3px),
			radial-gradient(circle at 10px calc(100% - 10px), #1f130a 2px, transparent 3px),
			radial-gradient(circle at calc(100% - 10px) calc(100% - 10px), #1f130a 2px, transparent 3px),
			linear-gradient(180deg, var(--wood) 0%, var(--wood-dark) 100%);
		border: 2px solid #2a1a0d;
		border-radius: var(--radius-sm);
		box-shadow:
			inset 0 1px 0 rgba(255, 220, 170, 0.12),
			var(--shadow-md);
	}

	.title {
		font-family: var(--font-family-heading);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		color: var(--carve);
		text-shadow: 0 -1px 1px rgba(0, 0, 0, 0.6);
	}

	.subtitle {
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
		color: #b89066;
	}
</style>
