<script lang="ts">
	// The realm slot of the application inspector renders the realm editor's
	// context-aware inspector (onboarding / resource / agent / graph / summary).
	import { lazy } from '#lib/core/lazy.js';
	import { realmInspector } from '#lib/widgets/realms/lazyInspector.js';
</script>

{#await lazy(realmInspector) then RealmInspector}
	<RealmInspector />
{/await}
