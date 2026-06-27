<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<div class="widget sign-steampunk">
	<span class="title">{str(props.title)}</span>
	{#if props.subtitle}
		<span class="subtitle">{str(props.subtitle)}</span>
	{/if}
</div>

<style>
	.sign-steampunk {
		--brass: #c9a14a;
		--brass-dark: #8a6d2c;
		--brass-light: #f0d488;
		width: fit-content;
		max-width: 75%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding: var(--spacing-md) var(--spacing-xl);
		text-align: center;
		background:
			radial-gradient(circle at 9px 9px, var(--brass-dark) 2.5px, transparent 3.5px),
			radial-gradient(circle at calc(100% - 9px) 9px, var(--brass-dark) 2.5px, transparent 3.5px),
			radial-gradient(circle at 9px calc(100% - 9px), var(--brass-dark) 2.5px, transparent 3.5px),
			radial-gradient(
				circle at calc(100% - 9px) calc(100% - 9px),
				var(--brass-dark) 2.5px,
				transparent 3.5px
			),
			linear-gradient(145deg, var(--brass-light) 0%, var(--brass) 45%, var(--brass-dark) 100%);
		border: 2px solid var(--brass-dark);
		border-radius: var(--radius-md);
		box-shadow:
			inset 0 1px 1px rgba(255, 255, 255, 0.4),
			var(--shadow-md);
	}

	.title {
		font-family: var(--font-family-heading);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		color: #3a2c10;
		text-shadow: 0 1px 0 rgba(255, 230, 160, 0.5);
	}

	.subtitle {
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
		color: #5a4720;
	}
</style>
