<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onMount } from 'svelte';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
	import { toolStore } from '#lib/stores/tools.svelte.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import { showApiError } from '#lib/stores/errorActions.js';
	import type { AttachmentKind } from '#lib/types/character.js';
	import type { Snowflake } from '#lib/types/core.js';
	import { Copy } from '@lucide/svelte';

	/** Called after any change the editor should save. */
	let { onchange }: { onchange: () => void } = $props();

	interface Option {
		id: Snowflake;
		name: string;
		tags: string[];
		description: string;
	}

	const lorebooks = $derived<Option[]>(
		(lorebookStore.metadataList ?? []).map((b) => ({
			id: b.id,
			name: b.name || t.studio.attachments.untitledLorebook,
			tags: [t.studio.attachments.entries({ count: b.entry_count })],
			description: b.description
		}))
	);
	const tools = $derived<Option[]>(
		(toolStore.loadedTools ?? []).map((t) => ({
			id: t.id,
			name: t.display_name || t.name,
			tags: t.display_name ? [t.name] : [],
			description: t.description
		}))
	);
	const widgets = $derived<Option[]>(
		(userWidgetStore.loadedWidgets ?? []).map((w) => ({
			id: w.id,
			name: w.name,
			tags: w.placement === 'widget' ? [w.placement, w.mode] : [w.placement],
			description: w.description
		}))
	);

	const groups: Array<{
		kind: AttachmentKind;
		ordered: boolean;
		options: () => Option[];
	}> = [
		{
			kind: 'lorebook',
			ordered: true,
			options: () => lorebooks
		},
		{
			kind: 'tool',
			ordered: false,
			options: () => tools
		},
		{
			kind: 'widget',
			ordered: true,
			options: () => widgets
		}
	];

	const impliedTools = $derived.by(() => {
		const out: Array<{ tool: Option | undefined; id: Snowflake; via: string }> = [];
		for (const widgetId of characterStore.attached('widget')) {
			const widget = userWidgetStore.byId(widgetId);
			if (!widget) continue;
			for (const id of widget.tools) {
				if (characterStore.isAttached('tool', id) || out.some((t) => t.id === id)) continue;
				out.push({ id, tool: tools.find((t) => t.id === id), via: widget.name });
			}
		}
		return out;
	});

	onMount(() => {
		if (!lorebookStore.metadataList) void lorebookStore.loadList();
		if (!toolStore.loadedTools) void toolStore.loadTools();
		if (!userWidgetStore.loadedWidgets) void userWidgetStore.loadWidgets();
	});

	function attach(kind: AttachmentKind, event: Event) {
		const select = event.currentTarget as HTMLSelectElement;
		const id = select.value;
		select.value = '';
		if (!id) return;
		characterStore.toggleAttachment(kind, id);
		onchange();
	}

	function detach(kind: AttachmentKind, id: Snowflake) {
		characterStore.toggleAttachment(kind, id);
		onchange();
	}

	function move(kind: AttachmentKind, index: number, dir: -1 | 1) {
		characterStore.moveAttachment(kind, index, dir);
		onchange();
	}

	let duplicating = $state<Snowflake | null>(null);
	async function duplicate(kind: AttachmentKind, id: Snowflake) {
		duplicating = id;
		try {
			await characterStore.duplicateAttachment(kind, id);
			if (kind === 'lorebook') await lorebookStore.loadList();
			if (kind === 'tool') await toolStore.loadTools();
			if (kind === 'widget') await userWidgetStore.loadWidgets();
		} catch (e) {
			showApiError(e, { title: t.studio.attachments.duplicateFailed });
		} finally {
			duplicating = null;
		}
	}
</script>

<h1>{t.studio.attachments.title}</h1>
<p class="hint">{t.studio.attachments.intro}</p>

{#each groups as group (group.kind)}
	{@const attached = characterStore.attached(group.kind)}
	{@const available = group.options().filter((o) => !attached.includes(o.id))}
	{@const text = t.studio.attachments.groups[group.kind]}
	<section class="group">
		<div class="group-head">
			<h2>{text.title}</h2>
			<select
				aria-label={text.attach}
				disabled={available.length === 0}
				onchange={(e) => attach(group.kind, e)}
			>
				<option value=""
					>{group.options().length === 0
						? text.none
						: available.length === 0
							? text.allAttached
							: text.attachPrompt}</option
				>
				{#each available as option (option.id)}
					<option value={option.id}>{option.name}</option>
				{/each}
			</select>
		</div>
		<p class="hint">{text.hint}</p>

		{#if attached.length === 0 && (group.kind !== 'tool' || impliedTools.length === 0)}
			<p class="empty">{text.noneAttached}</p>
		{/if}

		{#each attached as id, i (id)}
			{@const option = group.options().find((o) => o.id === id)}
			<div class="item-card">
				<div class="row">
					<div class="meta">
						<span class="name">{option?.name ?? text.unknown}</span>
						{#if group.kind === 'lorebook' && i === 0}
							<span class="tag main">{t.studio.attachments.main}</span>
						{/if}
						{#if option}
							{#each option.tags as tag (tag)}
								<span class="tag">{tag}</span>
							{/each}
						{:else}
							<span class="tag missing">{t.studio.attachments.missing}</span>
						{/if}
					</div>
					<div class="item-actions">
						{#if group.ordered}
							<button
								type="button"
								aria-label={t.studio.attachments.moveUp}
								disabled={i === 0}
								onclick={() => move(group.kind, i, -1)}>↑</button
							>
							<button
								type="button"
								aria-label={t.studio.attachments.moveDown}
								disabled={i === attached.length - 1}
								onclick={() => move(group.kind, i, 1)}>↓</button
							>
						{/if}
						<button
							type="button"
							{@attach tooltip(text.duplicateHint)}
							aria-label={t.studio.attachments.duplicateNamed({ name: option?.name ?? text.noun })}
							disabled={!option || duplicating !== null}
							onclick={() => duplicate(group.kind, id)}
						>
							<Copy size={14} />
						</button>
						<button
							type="button"
							class="danger"
							aria-label={t.studio.attachments.detachNamed({ name: option?.name ?? text.noun })}
							onclick={() => detach(group.kind, id)}>✕</button
						>
					</div>
				</div>
				{#if option?.description}
					<p class="desc">{option.description}</p>
				{/if}
			</div>
		{/each}

		{#if group.kind === 'tool'}
			{#each impliedTools as implied (implied.id)}
				<div class="item-card implied">
					<div class="row">
						<div class="meta">
							<span class="name"
								>{implied.tool?.name ?? t.studio.attachments.groups.tool.unknown}</span
							>
							<span class="tag">{t.studio.attachments.via({ name: implied.via })}</span>
						</div>
					</div>
				</div>
			{/each}
		{/if}
	</section>
{/each}

<style>
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		margin-top: var(--spacing-md);
	}
	.group-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
	}
	.group-head h2 {
		margin: 0;
		font-size: var(--font-size-md);
	}
	.group-head select {
		width: auto;
		max-width: 260px;
		font-size: var(--font-size-sm);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		margin: 0;
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		margin: 0;
	}
	.item-card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		background-color: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm);
	}
	.item-card.implied {
		background-color: transparent;
		border-style: dashed;
		color: var(--color-text-muted);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.meta {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
	}
	.implied .name {
		color: var(--color-text-secondary);
	}
	.tag {
		flex: 0 0 auto;
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		border-radius: var(--radius-full);
		padding: 0 var(--spacing-sm);
		font-size: var(--font-size-xs);
	}
	.tag.main {
		background-color: var(--color-primary);
		color: var(--color-text-white);
	}
	.tag.missing {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.desc {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.item-actions {
		display: flex;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
	}
	.item-actions button {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.item-actions button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.item-actions .danger:hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
</style>
