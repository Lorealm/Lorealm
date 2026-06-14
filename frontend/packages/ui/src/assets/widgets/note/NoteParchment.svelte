<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<div class="widget note-parchment">
	{#if props.title}
		<span class="title">{str(props.title)}</span>
	{/if}
	<div class="body">{str(props.content)}</div>
</div>

<style>
	.note-parchment {
		--ink: #4a3722;
		position: relative;
		width: fit-content;
		max-width: 75%;
		padding: var(--spacing-lg) var(--spacing-md);
		color: var(--ink);
		background: radial-gradient(120% 100% at 50% 0%, #f8efd8 0%, #f1e2c4 60%, #d8c098 100%);
		border: 1px solid #b89b6c;
		border-radius: var(--radius-sm);
		box-shadow:
			inset 0 0 24px rgba(120, 90, 50, 0.25),
			var(--shadow-md);
		font-family: var(--font-family-body-alt);
	}

	.title {
		display: block;
		margin-bottom: var(--spacing-xs);
		font-family: var(--font-family-heading);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}

	.body {
		font-style: italic;
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.note-parchment::after {
		content: '';
		position: absolute;
		right: -6px;
		bottom: -6px;
		width: 24px;
		height: 24px;
		border-radius: 50%;
		background: radial-gradient(circle at 35% 30%, #b5384a, #7c1f2c);
		box-shadow: var(--shadow-sm);
	}
</style>
