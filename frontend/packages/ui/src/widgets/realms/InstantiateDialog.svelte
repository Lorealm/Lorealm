<script lang="ts">
	import { ArrowLeft, ArrowRight, Play, X } from '@lucide/svelte';
	import DOMPurify from 'dompurify';
	import { marked } from 'marked';
	import type { InputSpec, InputSpecType } from '$lib/types/blueprint.js';
	import InputControl, { blankFor } from './InputControl.svelte';
	import type { RealmResourceKind } from '$lib/types/realm.js';
	import { ensureLoaded } from './resources.svelte.js';

	let {
		specs,
		introduction = null,
		busy = false,
		onbuild,
		oncancel
	}: {
		specs: InputSpec[];
		/** The author's markdown. Covers the first page when there is any. */
		introduction?: string | null;
		busy?: boolean;
		onbuild: (inputs: Record<string, unknown>) => void;
		oncancel: () => void;
	} = $props();

	let values = $state<Record<string, unknown>>({});

	let read = $state(false);
	const intro = $derived(
		introduction?.trim() ? DOMPurify.sanitize(marked.parse(introduction, { async: false })) : null
	);
	const onIntro = $derived(intro !== null && !read);

	function openLinksOutside(event: MouseEvent) {
		const link = (event.target as Element).closest('a');
		if (!link?.href) return;
		event.preventDefault();
		window.open(link.href, '_blank', 'noopener');
	}

	const referenced = $derived(specs.flatMap(kindsIn));

	$effect(() => {
		ensureLoaded(referenced);
	});

	// Seed each field so the form has something coherent before anyone touches it.
	$effect(() => {
		const seeded: Record<string, unknown> = {};
		for (const spec of specs) seeded[spec.name] = blankFor(spec);
		values = seeded;
	});

	function kindsIn(ty: InputSpecType): RealmResourceKind[] {
		switch (ty.type) {
			case 'reference':
				return ty.kind ? [ty.kind] : [];
			case 'list':
				return kindsIn(ty.of);
			case 'object':
				return ty.fields.flatMap(kindsIn);
			default:
				return [];
		}
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
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && oncancel()} />

<div
	class="overlay"
	role="presentation"
	onclick={(e) => e.currentTarget === e.target && oncancel()}
>
	<div
		class="dialog"
		class:wide={onIntro}
		role="dialog"
		aria-modal="true"
		aria-label="Build a realm"
	>
		<header>
			<h2>Build a realm</h2>
			<button type="button" class="close" aria-label="Cancel" onclick={oncancel}>
				<X size={14} />
			</button>
		</header>

		{#if onIntro}
			<!-- Sanitized with DOMPurify. -->
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			<div class="body intro" onclickcapture={openLinksOutside}>{@html intro}</div>
		{:else}
			<div class="body">
				{#each specs as spec (spec.name)}
					<div class="field">
						<span class="label">
							{spec.name}
							{#if !spec.required}<span class="optional">optional</span>{/if}
						</span>
						{#if spec.doc}<span class="doc">{spec.doc}</span>{/if}

						<InputControl
							ty={spec}
							value={values[spec.name]}
							ariaLabel={spec.name}
							onchange={(next) => (values[spec.name] = next)}
						/>

						<span class="declared">{spec.declared}</span>
					</div>
				{:else}
					<p class="nothing">This document declares no inputs.</p>
				{/each}
			</div>
		{/if}

		<footer>
			{#if intro !== null && !onIntro}
				<button type="button" class="back" onclick={() => (read = false)}>
					<ArrowLeft size={12} /> Introduction
				</button>
			{/if}
			{#if !onIntro && missing.length > 0}
				<span class="missing">
					Still needed: {missing.map((s) => s.name).join(', ')}
				</span>
			{/if}
			<span class="spacer"></span>
			<button type="button" onclick={oncancel}>Cancel</button>
			{#if onIntro && specs.length > 0}
				<button type="button" class="primary" onclick={() => (read = true)}>
					Continue <ArrowRight size={12} />
				</button>
			{:else}
				<button
					type="button"
					class="primary"
					disabled={busy || missing.length > 0}
					onclick={submit}
				>
					<Play size={12} />
					{busy ? 'Building…' : 'Build realm'}
				</button>
			{/if}
		</footer>
	</div>
</div>

<style>
	.overlay {
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
	.dialog.wide {
		width: min(680px, 92vw);
	}
	.intro {
		display: block;
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
		line-height: var(--line-height-relaxed, 1.6);
		overflow-wrap: anywhere;
	}
	.intro :global(:is(h1, h2, h3, h4)) {
		margin: var(--spacing-md) 0 var(--spacing-xs);
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h5);
	}
	.intro :global(h1) {
		font-size: var(--font-size-h4);
	}
	.intro :global(:is(p, ul, ol, pre, blockquote)) {
		margin: 0 0 var(--spacing-sm);
	}
	.intro :global(> :first-child) {
		margin-top: 0;
	}
	.intro :global(> :last-child) {
		margin-bottom: 0;
	}
	.intro :global(code) {
		font-family: var(--font-family-mono);
		font-size: 0.9em;
	}
	.intro :global(pre) {
		overflow-x: auto;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-input);
	}
	.intro :global(img) {
		max-width: 100%;
	}
	.intro :global(a) {
		color: var(--color-text-accent);
	}
	.doc {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		white-space: pre-line;
	}
	.declared {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
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
	footer .back {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
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
