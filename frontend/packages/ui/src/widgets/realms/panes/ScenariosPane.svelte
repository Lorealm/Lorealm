<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { BookOpen, Plus, Trash2 } from '@lucide/svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import { realmDraft } from '../draft.svelte.js';
	import { slug } from '../model.js';
	import { activeDrag, carriesAsset, dropTextInto } from '../drag.svelte.js';

	const incoming = $derived(activeDrag());
	const takesInput = $derived(incoming?.drag === 'input');

	let selected = $state<string | null>(null);

	const scenario = $derived(realmDraft.scenarios.find((s) => s.key === selected) ?? null);
	const index = $derived(realmDraft.scenarios.findIndex((s) => s.key === selected));

	function add() {
		selected = realmDraft.addScenario().key;
	}

	function remove(key: string) {
		const at = realmDraft.scenarios.findIndex((s) => s.key === key);
		realmDraft.removeScenario(key);
		if (selected === key) {
			selected = (realmDraft.scenarios[at] ?? realmDraft.scenarios[at - 1])?.key ?? null;
		}
	}
</script>

<div class="pane">
	<header>
		<h2>{t.realms.scenarios.title}</h2>
		<span class="hint">{t.realms.scenarios.hint}</span>
		<span class="spacer"></span>
		<button type="button" class="chip" onclick={add}>
			<Plus size={12} />
			{t.realms.scenarios.add}
		</button>
	</header>

	<div class="body">
		<div class="list">
			{#each realmDraft.scenarios as entry (entry.key)}
				<div class="row" class:active={entry.key === selected}>
					<button type="button" onclick={() => (selected = entry.key)}>
						<BookOpen size={12} />
						<span>{entry.name || t.realms.scenarios.untitled}</span>
					</button>
					<button
						type="button"
						class="remove"
						{@attach tooltip(t.realms.scenarios.remove)}
						aria-label={t.realms.scenarios.removeNamed(
							entry.name || t.realms.scenarios.fallbackName
						)}
						onclick={() => remove(entry.key)}
					>
						<Trash2 size={12} />
					</button>
				</div>
			{:else}
				<p class="empty">{t.realms.scenarios.none}</p>
			{/each}
		</div>

		<div class="editor">
			{#if scenario}
				{@const at = index}
				<label>
					<span>{t.realms.scenarios.name}</span>
					<input
						class:candidate={takesInput}
						class:incompatible={incoming !== null && !takesInput}
						value={scenario.name}
						placeholder={t.realms.scenarios.untitled}
						oninput={(e) => (realmDraft.scenarios[at].name = e.currentTarget.value)}
						ondragover={(e) => carriesAsset(e) && takesInput && e.preventDefault()}
						ondrop={(e) => {
							const next = dropTextInto(e);
							if (next !== null) realmDraft.scenarios[at].name = next;
						}}
					/>
				</label>
				<label>
					<span>{t.realms.scenarios.slug}</span>
					<input
						class="mono"
						value={scenario.slug}
						placeholder={slug(scenario.name) || t.realms.scenarios.slugPlaceholder}
						spellcheck="false"
						oninput={(e) => (realmDraft.scenarios[at].slug = e.currentTarget.value)}
					/>
				</label>

				<label>
					<span>{t.realms.scenarios.description}</span>
					<input
						class:candidate={takesInput}
						class:incompatible={incoming !== null && !takesInput}
						value={scenario.description}
						placeholder={t.realms.scenarios.descriptionPlaceholder}
						oninput={(e) => (realmDraft.scenarios[at].description = e.currentTarget.value)}
						ondragover={(e) => carriesAsset(e) && takesInput && e.preventDefault()}
						ondrop={(e) => {
							const next = dropTextInto(e);
							if (next !== null) realmDraft.scenarios[at].description = next;
						}}
					/>
				</label>

				<ContextField
					label={t.realms.scenarios.opening}
					value={scenario.opening}
					onchange={(next) => (realmDraft.scenarios[at].opening = next)}
					dialect="template"
					profile="realm_template"
					surface="sunken"
					fill
					placeholder={t.realms.scenarios.openingPlaceholder}
					hint={t.realms.scenarios.openingHint}
				/>
			{:else}
				<p class="nothing">
					{realmDraft.scenarios.length === 0 ? t.realms.scenarios.empty : t.realms.scenarios.pick}
				</p>
			{/if}
		</div>
	</div>
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
	header {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		gap: var(--spacing-sm);
		height: var(--height-header);
		padding: 0 var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.spacer {
		flex: 1 1 auto;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 3px var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.chip:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}

	.body {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	.list {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 180px;
		min-height: 0;
		padding: var(--spacing-xs);
		gap: 2px;
		overflow-y: auto;
		border-right: 1px solid var(--color-border-subtle);
	}
	.row {
		display: flex;
		align-items: center;
		border-radius: var(--radius-sm);
	}
	.row:hover {
		background-color: var(--color-hover);
	}
	.row.active {
		background-color: var(--color-selected);
	}
	.row > button:first-child {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-secondary);
		text-align: left;
	}
	.row.active > button:first-child {
		color: var(--color-text-accent);
	}
	.row span {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
	}
	.remove {
		flex: 0 0 auto;
		display: flex;
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
		opacity: 0;
	}
	.row:hover .remove,
	.remove:focus-visible {
		opacity: 1;
	}
	.remove:hover {
		color: var(--color-error-text);
	}

	.editor {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	label > span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.candidate {
		border-color: var(--color-border-active) !important;
		border-style: dashed !important;
	}
	.incompatible {
		opacity: 0.35;
		filter: grayscale(1);
	}
	.mono {
		font-family: var(--font-family-mono);
	}

	.empty,
	.nothing {
		margin: 0;
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.nothing {
		margin: auto;
		max-width: 26em;
		text-align: center;
	}

	@media (hover: none) {
		.remove {
			opacity: 1;
		}
	}
</style>
