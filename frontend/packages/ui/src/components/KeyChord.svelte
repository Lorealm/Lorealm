<script lang="ts">
	import { formatChord, type Chord } from '#lib/keys/chord.js';

	let { chord }: { chord: Chord } = $props();
</script>

<kbd>{formatChord(chord)}</kbd>

<style>
	kbd {
		display: inline-flex;
		align-items: center;
		padding: 2px var(--spacing-sm);
		border: 1px solid var(--color-border-medium);
		border-bottom-width: 2px;
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-elevated-2);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		white-space: nowrap;
	}
</style>
