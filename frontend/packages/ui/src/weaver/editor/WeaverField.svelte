<script module lang="ts">
	/** Lazy shell for {@link WeaverFieldView}. See {@link WeaverArea} for why. */
	const view = () => import('./WeaverFieldView.svelte');
</script>

<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import { lazy } from '#lib/core/lazy.js';

	const props: ComponentProps<typeof import('./WeaverFieldView.svelte').default> = $props();
</script>

{#await lazy(view)}
	<div class="placeholder"></div>
{:then WeaverFieldView}
	<WeaverFieldView {...props} />
{/await}

<style>
	.placeholder {
		min-height: calc(1.5em + 2 * var(--spacing-sm));
	}
</style>
