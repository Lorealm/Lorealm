<script lang="ts">
	let {
		values,
		onchange,
		placeholder = '',
		addLabel = '+ Add',
		ariaLabel
	}: {
		values: string[] | undefined;
		onchange: (next: string[] | undefined) => void;
		placeholder?: string;
		addLabel?: string;
		ariaLabel?: string;
	} = $props();

	const items = $derived(values ?? []);

	function emit(next: string[]) {
		onchange(next.length > 0 ? next : undefined);
	}
	function update(i: number, v: string) {
		const next = [...items];
		next[i] = v;
		emit(next);
	}
	function remove(i: number) {
		emit(items.filter((_, j) => j !== i));
	}
	function add() {
		emit([...items, '']);
	}
</script>

<div class="string-list" role="group" aria-label={ariaLabel}>
	{#each items as item, i (i)}
		<div class="row">
			<input
				type="text"
				{placeholder}
				value={item}
				oninput={(e) => update(i, e.currentTarget.value)}
			/>
			<button type="button" class="remove" aria-label="Remove" onclick={() => remove(i)}>×</button>
		</div>
	{/each}
	<button type="button" class="add" onclick={add}>{addLabel}</button>
</div>

<style>
	.string-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.row {
		display: flex;
		gap: var(--spacing-xs);
		align-items: center;
	}
	.row input {
		flex: 1 1 auto;
		min-width: 0;
	}
	.remove {
		flex: 0 0 auto;
		line-height: 1;
	}
	.add {
		align-self: flex-start;
		font-size: var(--font-size-sm);
	}
</style>
