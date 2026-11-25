<script lang="ts">
	import ConfirmDelete from '#lib/components/ConfirmDelete.svelte';
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { connectionStore } from '#lib/stores/connection.svelte.js';
	import type { ConnectionProfile } from '#lib/types/connection.js';
	import type { Snowflake } from '#lib/types/core.js';
	import { Star, Trash2 } from '@lucide/svelte';

	let activeProfileId = $derived(connectionStore.activeProfile);

	let {
		profiles,
		selectedId,
		onSelect,
		onActivate,
		onDelete
	}: {
		profiles: ConnectionProfile[];
		selectedId: Snowflake | null;
		onSelect: (id: Snowflake) => void;
		onActivate: (id: Snowflake) => void;
		onDelete: (id: Snowflake) => void;
	} = $props();
</script>

{#each profiles as profile (profile.id)}
	<div
		class="profile-item"
		class:selected={profile.id === selectedId}
		class:is-active={profile.id === activeProfileId}
	>
		<button class="profile-open" type="button" onclick={() => onSelect(profile.id)}>
			<span class="profile-text">
				<span class="profile-name">{profile.name || t.connections.untitledProfile}</span>
				<span class="profile-sub"
					>{profile.provider} · {profile.model || t.connections.noModel}</span
				>
			</span>
		</button>
		<button
			class="icon-button active-toggle"
			class:on={profile.id === activeProfileId}
			type="button"
			{@attach tooltip(
				profile.id === activeProfileId ? t.connections.activeConnection : t.connections.setActive
			)}
			aria-pressed={profile.id === activeProfileId}
			aria-label={profile.id === activeProfileId
				? t.connections.isActive({ name: profile.name })
				: t.connections.makeActive({ name: profile.name })}
			onclick={() => onActivate(profile.id)}
		>
			<Star size={14} fill={profile.id === activeProfileId ? 'currentColor' : 'none'} />
		</button>
		<ConfirmDelete
			prompt={t.connections.confirmDelete({ name: profile.name || t.connections.untitledProfile })}
			onConfirm={() => onDelete(profile.id)}
		>
			{#snippet trigger(arm)}
				<button
					class="icon-button delete"
					type="button"
					{@attach tooltip(t.connections.deleteProfile)}
					aria-label={t.connections.deleteNamed({ name: profile.name })}
					onclick={arm}
				>
					<Trash2 size={14} />
				</button>
			{/snippet}
		</ConfirmDelete>
	</div>
{/each}

<style>
	.profile-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-sm);
	}
	.profile-item:hover {
		background-color: var(--color-hover);
	}
	.profile-item.selected {
		background-color: var(--color-selected);
	}
	.profile-item.is-active {
		box-shadow: inset 2px 0 0 var(--color-border-active);
	}
	.profile-open {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		background: transparent;
		padding: var(--spacing-sm);
		text-align: left;
	}
	.profile-text {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.profile-name {
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.profile-sub {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon-button {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-muted);
	}
	.icon-button:hover {
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-accent);
	}
	.icon-button:focus-visible {
		outline: none;
		box-shadow: 0 0 0 2px var(--color-focus);
	}
	.active-toggle.on {
		color: var(--color-primary);
	}
	.delete:hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.delete:last-child {
		margin-right: var(--spacing-xs);
	}
</style>
