<script module lang="ts">
	/**
	 * CodeMirror and the weaver language support are a third of the app's
	 * JavaScript, and most sessions never open an editor. This shell holds the
	 * public name and the layout box; {@link WeaverAreaView} — which carries the
	 * actual dependency — arrives in its own chunk on first render.
	 */
	const view = () => import('./WeaverAreaView.svelte');
</script>

<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import { lazy } from '#lib/core/lazy.js';

	const props: ComponentProps<typeof import('./WeaverAreaView.svelte').default> = $props();
</script>

{#await lazy(view)}
	<!-- Same box as the real editor, so nothing shifts when it lands. -->
	<div
		class="weaverArea"
		class:fill={props.fill}
		style:min-height={props.fill
			? '100%'
			: `calc(${props.rows ?? 6} * 1.5em + 2 * var(--spacing-sm))`}
	></div>
{:then WeaverAreaView}
	<WeaverAreaView {...props} />
{/await}

<style>
	.weaverArea {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.weaverArea.fill {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
