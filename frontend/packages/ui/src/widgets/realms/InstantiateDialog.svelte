<script lang="ts">
	import { Play, X } from '@lucide/svelte';
	import type { InputSpec, InputSpecType } from '$lib/types/blueprint.js';
	import type { RealmResourceKind } from '$lib/types/realm.js';
	import { ensureLoaded, itemsOf } from './resources.svelte.js';

	let {
		specs,
		busy = false,
		onbuild,
		oncancel
	}: {
		specs: InputSpec[];
		busy?: boolean;
		onbuild: (inputs: Record<string, unknown>) => void;
		oncancel: () => void;
	} = $props();

	let values = $state<Record<string, unknown>>({});

	const referenced = $derived(
		specs.flatMap((spec) => {
			const kind = refKind(spec);
			return kind ? [kind] : [];
		})
	);

	$effect(() => {
		ensureLoaded(referenced);
	});

	// Seed each field so the form has something coherent before anyone touches it.
	$effect(() => {
		const seeded: Record<string, unknown> = {};
		for (const spec of specs) {
			seeded[spec.name] = spec.type === 'list' ? [] : spec.type === 'bool' ? false : '';
		}
		values = seeded;
	});

	function refKind(ty: InputSpecType): RealmResourceKind | null {
		if (ty.type === 'reference') return ty.kind;
		if (ty.type === 'list' && ty.of.type === 'reference') return ty.of.kind;
		return null;
	}

	/** Required fields must be answered; the rest fall back to their default. */
	const missing = $derived(
		specs.filter((spec) => {
			if (!spec.required) return false;
			const value = values[spec.name];
			return Array.isArray(value) ? value.length === 0 : value === '' || value === undefined;
		})
	);

	function submit() {
		const supplied: Record<string, unknown> = {};
		for (const [name, value] of Object.entries(values)) {
			if (value === '' || (Array.isArray(value) && value.length === 0)) continue;
			supplied[name] = value;
		}
		onbuild(supplied);
	}

	function toggle(name: string, id: string, on: boolean) {
		const held = (values[name] as string[]) ?? [];
		values[name] = on ? [...held, id] : held.filter((x) => x !== id);
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && oncancel()} />

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => e.currentTarget === e.target && oncancel()}
>
	<div class="dialog" role="dialog" aria-modal="true" aria-label="Build a realm">
		<header>
			<h2>Build a realm</h2>
			<button type="button" class="close" aria-label="Cancel" onclick={oncancel}>
				<X size={14} />
			</button>
		</header>

		<div class="body">
			{#each specs as spec (spec.name)}
				{@const kind = refKind(spec)}
				<div class="field">
					<span class="label">
						{spec.name}
						{#if !spec.required}<span class="optional">optional</span>{/if}
					</span>

					{#if spec.type === 'list' && kind}
						<div class="checks">
							{#each itemsOf(kind) as item (item.id)}
								<label>
									<input
										type="checkbox"
										checked={((values[spec.name] as string[]) ?? []).includes(item.id)}
										onchange={(e) => toggle(spec.name, item.id, e.currentTarget.checked)}
									/>
									<span>{item.name}</span>
								</label>
							{:else}
								<p class="none">You own no {kind}s to pick from.</p>
							{/each}
						</div>
					{:else if kind}
						<select
							value={(values[spec.name] as string) ?? ''}
							onchange={(e) => (values[spec.name] = e.currentTarget.value)}
						>
							<option value="">— pick one —</option>
							{#each itemsOf(kind) as item (item.id)}
								<option value={item.id}>{item.name}</option>
							{/each}
						</select>
					{:else if spec.type === 'enum'}
						<select
							value={(values[spec.name] as string) ?? ''}
							onchange={(e) => (values[spec.name] = e.currentTarget.value)}
						>
							<option value="">— default —</option>
							{#each spec.variants as variant (variant)}
								<option value={variant}>{variant}</option>
							{/each}
						</select>
					{:else if spec.type === 'bool'}
						<label class="check">
							<input
								type="checkbox"
								checked={values[spec.name] === true}
								onchange={(e) => (values[spec.name] = e.currentTarget.checked)}
							/>
							<span>Yes</span>
						</label>
					{:else if spec.type === 'number'}
						<input
							type="number"
							value={(values[spec.name] as string) ?? ''}
							onchange={(e) =>
								(values[spec.name] =
									e.currentTarget.value === '' ? '' : Number(e.currentTarget.value))}
						/>
					{:else}
						<input
							type="text"
							value={(values[spec.name] as string) ?? ''}
							onchange={(e) => (values[spec.name] = e.currentTarget.value)}
						/>
					{/if}

					<span class="declared">{spec.declared}</span>
				</div>
			{:else}
				<p class="nothing">This document declares no inputs.</p>
			{/each}
		</div>

		<footer>
			{#if missing.length > 0}
				<span class="missing">
					Still needed: {missing.map((s) => s.name).join(', ')}
				</span>
			{/if}
			<span class="spacer"></span>
			<button type="button" onclick={oncancel}>Cancel</button>
			<button type="button" class="primary" disabled={busy || missing.length > 0} onclick={submit}>
				<Play size={12} />
				{busy ? 'Building…' : 'Build realm'}
			</button>
		</footer>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: var(--color-bg-overlay);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.dialog {
		display: flex;
		flex-direction: column;
		width: min(520px, 92vw);
		max-height: 80vh;
		background-color: var(--color-bg-modal);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-xl);
	}
	header {
		display: flex;
		align-items: center;
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	h2 {
		flex: 1 1 auto;
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h5);
		font-weight: var(--font-weight-semibold);
	}
	.close {
		display: flex;
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-muted);
	}
	.close:hover {
		color: var(--color-text-accent);
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		overflow-y: auto;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.label {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
	}
	.optional {
		padding: 1px 6px;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-muted);
		font-family: var(--font-family-body);
		font-size: var(--font-size-xs);
	}
	.declared {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.checks {
		display: flex;
		flex-direction: column;
		gap: 2px;
		max-height: 12em;
		overflow-y: auto;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.checks label,
	.check {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.none {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.nothing {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	footer {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-top: 1px solid var(--color-border-subtle);
	}
	.missing {
		color: var(--color-warning-text);
		font-size: var(--font-size-xs);
	}
	.spacer {
		flex: 1 1 auto;
	}
	footer button {
		padding: var(--spacing-xs) var(--spacing-md);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	footer .primary {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
	footer .primary:disabled {
		opacity: 0.5;
	}
</style>
