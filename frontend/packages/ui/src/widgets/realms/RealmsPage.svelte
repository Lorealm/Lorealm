<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import RealmList from './RealmList.svelte';
	import RealmView from './RealmView.svelte';
</script>

{#if realmStore.realm}
	<RealmView />
{:else}
	<RealmList />
{/if}
