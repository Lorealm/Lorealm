<script lang="ts">
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import type { ConnectionProfile } from '$lib/types/connection.js';
	import type { Snowflake } from '$lib/types/core.js';
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
				<span class="profile-name">{profile.name || 'Untitled profile'}</span>
				<span class="profile-sub">{profile.provider} · {profile.model || 'no model'}</span>
			</span>
		</button>
		<button
			class="icon-button active-toggle"
			class:on={profile.id === activeProfileId}
			type="button"
			title={profile.id === activeProfileId ? 'Active connection' : 'Set as active connection'}
			aria-pressed={profile.id === activeProfileId}
			aria-label={profile.id === activeProfileId
				? `${profile.name} is the active connection`
				: `Set ${profile.name} as the active connection`}
			onclick={() => onActivate(profile.id)}
		>
			<Star size={14} fill={profile.id === activeProfileId ? 'currentColor' : 'none'} />
		</button>
		<button
			class="icon-button delete"
			type="button"
			title="Delete profile"
			aria-label={`Delete ${profile.name}`}
			onclick={() => onDelete(profile.id)}
		>
			<Trash2 size={14} />
		</button>
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
