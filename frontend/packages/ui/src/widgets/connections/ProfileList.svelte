<script lang="ts">
	import { connectionStore } from '$lib/stores/connection.svelte.js';
	import type { ConnectionProfile } from '$lib/types/connection.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { X } from '@lucide/svelte';

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

<ul class="profile-list">
	{#each profiles as profile (profile.id)}
		<li class="row" class:active={profile.id === selectedId}>
			<button class="select" type="button" onclick={() => onSelect(profile.id)}>
				<span class="name">{profile.name || 'Untitled profile'}</span>
				<span class="sub">{profile.provider} · {profile.model || 'no model'}</span>
			</button>
			<button
				class="activate"
				type="button"
				title="Set as active connection"
				aria-label="Activate profile"
				onclick={() => onActivate(profile.id)}>{profile.id === activeProfileId ? '★' : '☆'}</button
			>
			<button
				class="delete"
				type="button"
				aria-label="Delete profile"
				onclick={() => onDelete(profile.id)}><X size={16} /></button
			>
		</li>
	{:else}
		<li class="none">No profiles yet.</li>
	{/each}
</ul>

<style>
	.profile-list {
		list-style: none;
		margin: 0;
		padding: var(--spacing-xs) 0;
		overflow-y: auto;
		flex: 1 1 auto;
		min-height: 0;
	}
	.row {
		display: flex;
		align-items: stretch;
	}
	.row.active {
		background: var(--color-border-subtle);
	}
	.select {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		background: none;
		border: none;
		cursor: pointer;
	}
	.name {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.sub {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--font-size-sm);
		opacity: 0.6;
	}
	.activate {
		flex: 0 0 auto;
		padding: 0 var(--spacing-sm);
		background: none;
		border: none;
		cursor: pointer;
		opacity: 0.45;
		color: var(--color-text-secondary);
	}
	.row:hover .activate,
	.row.active .activate {
		opacity: 0.8;
	}
	.activate:hover {
		opacity: 1;
		color: var(--color-text-accent);
	}
	.delete {
		flex: 0 0 auto;
		padding: 0 var(--spacing-sm);
		background: none;
		border: none;
		cursor: pointer;
		opacity: 0;
	}
	.row:hover .delete,
	.row.active .delete {
		opacity: 0.6;
	}
	.delete:hover {
		opacity: 1;
	}
	.none {
		padding: var(--spacing-md);
		font-size: var(--font-size-sm);
		opacity: 0.6;
	}
</style>
