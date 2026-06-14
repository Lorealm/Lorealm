<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<div class="widget note-arcane">
	{#if props.title}
		<span class="title">{str(props.title)}</span>
	{/if}
	<div class="body">{str(props.content)}</div>
</div>

<style>
	.note-arcane {
		--rune: #b890ff;
		--rune-glow: #7b4dff;
		position: relative;
		width: fit-content;
		max-width: 75%;
		padding: var(--spacing-lg) var(--spacing-md);
		color: #e7dcff;
		background: linear-gradient(160deg, #221a3a 0%, #15102a 100%);
		border: 1px solid color-mix(in srgb, var(--rune-glow) 50%, transparent);
		border-radius: var(--radius-md);
		box-shadow:
			inset 0 0 20px color-mix(in srgb, var(--rune-glow) 18%, transparent),
			0 0 14px color-mix(in srgb, var(--rune-glow) 25%, transparent),
			var(--shadow-md);
		font-family: var(--font-family-body-alt);
	}

	.title {
		display: block;
		margin-bottom: var(--spacing-xs);
		font-family: var(--font-family-heading);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		color: var(--rune);
		text-shadow: 0 0 8px var(--rune-glow);
	}

	.body {
		font-style: italic;
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.note-arcane::before {
		content: '✦';
		position: absolute;
		top: var(--spacing-xs);
		right: var(--spacing-sm);
		color: var(--rune);
		text-shadow: 0 0 8px var(--rune-glow);
		opacity: 0.8;
		animation: pulse 3s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.5;
		}
		50% {
			opacity: 0.9;
		}
	}
</style>
