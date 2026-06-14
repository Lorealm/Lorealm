<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();

	const color = (() => {
		const c = str(props.color);
		return /^#[0-9a-fA-F]{3,8}$/.test(c) ? c : '#34e2e2';
	})();
</script>

<div class="widget sign-neon" style="--tube: {color};">
	<span class="text">{str(props.text)}</span>
</div>

<style>
	.sign-neon {
		width: fit-content;
		max-width: 75%;
		padding: var(--spacing-md) var(--spacing-xl);
		background: #15131c;
		border: 1px solid color-mix(in srgb, var(--tube) 40%, #15131c);
		border-radius: var(--radius-md);
		box-shadow:
			0 0 12px color-mix(in srgb, var(--tube) 35%, transparent),
			var(--shadow-md);
	}

	.text {
		font-family: var(--font-family-heading);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		color: #fff;
		text-shadow:
			0 0 4px var(--tube),
			0 0 10px var(--tube),
			0 0 20px var(--tube);
		animation: flicker 4s infinite;
	}

	@keyframes flicker {
		0%,
		92%,
		100% {
			opacity: 1;
		}
		93% {
			opacity: 0.7;
		}
		95% {
			opacity: 1;
		}
		96% {
			opacity: 0.85;
		}
	}
</style>
