<script module lang="ts">
	/** Lazy shell for {@link HtmlEditorView}. */
	const view = () => import('./HtmlEditorView.svelte');
</script>

<script lang="ts">
	import type { ComponentProps } from 'svelte';
	import { lazy } from '#lib/core/lazy.js';

	const props: ComponentProps<typeof import('./HtmlEditorView.svelte').default> = $props();
</script>

{#await lazy(view)}
	<div class="editor"></div>
{:then HtmlEditorView}
	<HtmlEditorView {...props} />
{/await}

<style>
	.editor {
		flex: 1;
		min-height: 0;
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border-subtle);
	}
</style>
