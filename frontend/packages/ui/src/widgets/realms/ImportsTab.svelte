<script lang="ts">
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { ImportKind } from '$lib/types/realm.js';
	import {
		BookOpen,
		FileText,
		LayoutDashboard,
		Pencil,
		Plug,
		User,
		CircleUser,
		Wrench,
		X
	} from '@lucide/svelte';
	import Badge from './Badge.svelte';
	import { catalogItem, IMPORT_KINDS, IMPORT_LABEL, realmEditor } from './realmEditor.svelte.js';

	const ICON = {
		character: User,
		lorebook: BookOpen,
		tool: Wrench,
		template: FileText,
		persona: CircleUser,
		connection_profile: Plug,
		widget: LayoutDashboard
	};

	const imports = $derived(realmStore.realm?.definition.imports ?? []);

	function idsOf(kind: ImportKind): Snowflake[] {
		return imports.filter((r) => r.kind === kind).map((r) => r.resource_id);
	}

	async function remove(kind: ImportKind, id: Snowflake) {
		const users = realmStore.importUsers(kind, id);
		if (users.length > 0) {
			showError(
				`Still used by ${users.map((a) => `"${a.name}"`).join(', ')}. Unbind it there first.`,
				{
					title: 'Import in use',
					durationMs: 5000
				}
			);
			return;
		}
		await realmStore.setImports(imports.filter((r) => !(r.kind === kind && r.resource_id === id)));
		if (realmEditor.selectedImportKey === `${kind}:${id}`) realmEditor.selectedImportKey = null;
	}
</script>

<div class="stage">
	{#each IMPORT_KINDS as kind (kind)}
		{@const ids = idsOf(kind)}
		{@const Icon = ICON[kind]}
		<section>
			<header>
				<div class="heading">
					<Icon size={16} color="var(--color-text-secondary)" />
					<h3>{IMPORT_LABEL[kind]}</h3>
					<Badge tone="neutral">{ids.length}</Badge>
				</div>
				<button type="button" class="secondary" onclick={() => (realmEditor.pickerKind = kind)}>
					+ Add
				</button>
			</header>

			{#if ids.length > 0}
				<div class="grid">
					{#each ids as id (id)}
						{@const item = catalogItem(kind, id)}
						{@const selected = realmEditor.selectedImportKey === `${kind}:${id}`}
						<div
							class="card"
							class:selected
							role="button"
							tabindex="0"
							onclick={() => realmEditor.selectImport(kind, id)}
							onkeydown={(e) => {
								if (e.key === 'Enter' || e.key === ' ') realmEditor.selectImport(kind, id);
							}}
						>
							<div class="cardBody">
								<div class="name">{item?.name ?? `#${id}`}</div>
								{#if item?.description}
									<div class="desc">{item.description}</div>
								{/if}
							</div>
							<div class="cardActions">
								{#if kind !== 'connection_profile'}
									<button
										type="button"
										class="icon"
										title="Edit in Studio"
										onclick={(e) => {
											e.stopPropagation();
											studioStore.editResource(kind, id);
										}}
									>
										<Pencil size={14} />
									</button>
								{/if}
								<button
									type="button"
									class="icon"
									title="Remove import"
									onclick={(e) => {
										e.stopPropagation();
										void remove(kind, id);
									}}
								>
									<X size={14} />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="empty">No {IMPORT_LABEL[kind].toLowerCase()} imported yet.</div>
			{/if}
		</section>
	{/each}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-2xl);
	}

	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--spacing-sm);
	}

	.heading {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	h3 {
		margin: 0;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
		gap: var(--spacing-md);
	}

	.card {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		cursor: pointer;
		text-align: left;
	}

	.card:hover {
		border-color: var(--color-border-medium);
	}

	.card.selected {
		border-color: var(--color-border-active);
	}

	.cardBody {
		min-width: 0;
	}

	.name {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-base);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.desc {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		margin-top: 2px;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.cardActions {
		display: flex;
		gap: 4px;
		flex-shrink: 0;
	}

	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background: none;
		border: none;
		color: var(--color-text-muted);
	}

	.icon:hover {
		color: var(--color-text-primary);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
</style>
