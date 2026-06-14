<script lang="ts">
	import { str } from '../util.js';
	let { props }: { props: Record<string, unknown> } = $props();
</script>

<div class="widget note">{str(props.content)}</div>

<style>
	.widget {
		max-width: 75%;
	}

	.note {
		position: relative;
		background-color: var(--color-bg-elevated);
		background-image: var(--gradient-subtle);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-sm);
		padding: var(--spacing-lg) var(--spacing-md);
		font-family: var(--font-family-body-alt);
		font-style: italic;
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		transform: rotate(-0.5deg);
		box-shadow: var(--shadow-md);
		transition:
			transform var(--transition-normal) var(--easing-default),
			box-shadow var(--transition-normal) var(--easing-default);
	}

	.note::before {
		content: '';
		position: absolute;
		top: -9px;
		left: 50%;
		width: 60px;
		height: 18px;
		transform: translateX(-50%) rotate(1.5deg);
		background: var(--color-primary-light);
		opacity: 0.3;
		border-radius: var(--radius-sm);
	}

	.note:hover {
		transform: rotate(0deg);
		box-shadow: var(--shadow-lg);
	}
</style>
