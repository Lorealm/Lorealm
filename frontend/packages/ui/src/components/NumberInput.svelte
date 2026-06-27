<script lang="ts">
	// Number field with custom, themeable spinner buttons. The native spinner
	// draws a black glyph with sub-borders that don't fit a dark UI, so we strip
	// it and drive changes through stepUp()/stepDown(). Controlled via the same
	// value/onchange contract as the other field components; emits '' for an empty
	// field so callers map it to undefined/null/0 per their field's rules. Owns
	// parsing — non-numeric input is ignored (the guard the old setNum had).
	let {
		value,
		onchange,
		min,
		max,
		step = 1,
		placeholder = '',
		width,
		ariaLabel
	}: {
		value: number | '';
		onchange: (next: number | '') => void;
		min?: number;
		max?: number;
		step?: number;
		placeholder?: string;
		/** Optional fixed width (e.g. "7rem") for compact inline use; fills its
		 *  container when omitted. */
		width?: string;
		ariaLabel?: string;
	} = $props();

	let input = $state<HTMLInputElement>();

	function nudge(dir: 1 | -1) {
		if (!input) return;
		if (dir > 0) input.stepUp();
		else input.stepDown();
		onchange(input.value === '' ? '' : input.valueAsNumber);
	}
	function onInput(e: Event & { currentTarget: HTMLInputElement }) {
		const raw = e.currentTarget.value;
		if (raw === '') return onchange('');
		const n = Number(raw);
		if (Number.isFinite(n)) onchange(n);
	}
</script>

<div class="number-field" style:width>
	<input
		bind:this={input}
		type="number"
		{min}
		{max}
		{step}
		{placeholder}
		aria-label={ariaLabel}
		{value}
		oninput={onInput}
	/>
	<div class="spinner">
		<button type="button" tabindex="-1" aria-label="Increment" onclick={() => nudge(1)}>▲</button>
		<button type="button" tabindex="-1" aria-label="Decrement" onclick={() => nudge(-1)}>▼</button>
	</div>
</div>

<style>
	.number-field {
		position: relative;
		display: flex;
	}
	.number-field input {
		flex: 1 1 auto;
		min-width: 0;
		padding-right: calc(var(--spacing-sm) + 1.1rem);
		-moz-appearance: textfield;
		appearance: textfield;
	}
	.number-field input::-webkit-inner-spin-button,
	.number-field input::-webkit-outer-spin-button {
		-webkit-appearance: none;
		appearance: none;
		margin: 0;
	}
	.spinner {
		position: absolute;
		right: var(--spacing-xs);
		top: 50%;
		transform: translateY(-50%);
		display: flex;
		flex-direction: column;
		justify-content: center;
	}
	.spinner button {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		width: 1rem;
		height: 0.85rem;
		line-height: 1;
		font-size: 0.55rem;
		background: none;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
	}
	.spinner button:hover {
		background: none;
		color: var(--color-text-accent);
	}
</style>
