<script module lang="ts">
	/** Lazy shell for {@link WeaverEditorView}. See {@link WeaverArea} for why. */
	const view = () => import('./WeaverEditorView.svelte');
</script>

<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import { lazy } from '$lib/core/lazy.js';

	const props: ComponentProps<typeof import('./WeaverEditorView.svelte').default> = $props();
</script>

{#await lazy(view)}
	<div class="editor"></div>
{:then WeaverEditorView}
	<WeaverEditorView {...props} />
{/await}

<style>
	.editor {
		display: flex;
		flex: 1 0;
		min-height: 50vh;
		height: 100%;
		overflow: hidden;
		border-radius: 8px;
		border: 1px solid var(--color-border-subtle);
	}
</style>
