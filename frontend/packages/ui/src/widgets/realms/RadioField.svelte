<script lang="ts" module>
	export const ResourceFilter = {
		All: 0,
		Character: 1 << 0,
		Lorebook: 1 << 1,
		Template: 1 << 2
		/** TODO: extend with all possible resources */
	} as const;

	export type ResourceFilter = number;
</script>

<script lang="ts">
	import { CircleDot } from '@lucide/svelte';

	let { filter }: { filter: ResourceFilter } = $props();
</script>

<div>
	<div class="radio-field">
		<div class="radio-content">
			<span>text</span>
		</div>
		<div class="radio-icon">
			<CircleDot size={14} />
		</div>
	</div>
</div>

<style>
	.radio-field {
		display: flex;
		width: 100%;
		padding: var(--spacing-xs) var(--spacing-sm);
		background-color: var(--color-bg-panel);
		align-items: center;
		border-radius: var(--radius-sm);
		border: 1px solid var(--color-border-subtle);
		color: var(--color-text-secondary);
	}
	.radio-field:hover {
		background-color: var(--color-bg-elevated);
		color: var(--color-text-primary);
		border-color: var(--color-border-active);
	}
	.radio-content {
		display: flex;
		flex: 1;
	}
	.radio-icon {
		display: flex;
		margin-left: auto;
		align-items: center;
		border-left: 1px solid var(--color-border-strong);
		padding-left: var(--radius-md);
	}
</style>
