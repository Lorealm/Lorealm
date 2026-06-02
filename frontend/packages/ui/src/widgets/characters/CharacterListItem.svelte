<script lang="ts">
	import TransportImage from '$lib/components/TransportImage.svelte';
	import type { CharacterSummary } from '$lib/types/character.js';

	let { summary }: { summary: CharacterSummary } = $props();
	let imageEndpoint = $derived(`/api/v1/characters/${summary.id}/image`);
</script>

<div class="avatar">
	<TransportImage src={imageEndpoint} alt={summary.name} />
</div>

<style>
	.avatar {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		overflow: hidden;
		flex-shrink: 0; /* so the flex row doesn't squash it */
	}
</style>
