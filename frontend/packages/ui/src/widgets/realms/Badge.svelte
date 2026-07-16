<script lang="ts">
	import type { Snippet } from 'svelte';

	type Tone = 'accent' | 'neutral' | 'info' | 'warning' | 'success';

	let { tone = 'neutral', children }: { tone?: Tone; children: Snippet } = $props();
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		font-size: var(--font-size-xs);
		font-weight: 600;
		line-height: 1;
		padding: 3px 7px;
		border-radius: var(--radius-full);
		white-space: nowrap;
	}

	.neutral {
		color: var(--color-text-secondary);
		background: var(--color-bg-elevated-2);
	}

	.accent {
		color: var(--color-text-accent);
		background: var(--color-selected);
	}

	.info {
		color: var(--color-info);
		background: color-mix(in srgb, var(--color-info) 15%, transparent);
	}

	.warning {
		color: var(--color-warning-text);
		background: color-mix(in srgb, var(--color-warning) 15%, transparent);
	}

	.success {
		color: var(--color-success-text);
		background: color-mix(in srgb, var(--color-success) 15%, transparent);
	}
</style>
