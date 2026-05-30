<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';

	let { id }: { id: string } = $props();
</script>

<button onclick={() => studioStore.close()}>← Back</button>
<h2>Editing lorebook: {id}</h2>
<!-- lorebook-specific editor (weaver-lang, entries, activation) goes here -->
