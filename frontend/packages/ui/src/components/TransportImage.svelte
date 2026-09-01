<script lang="ts">
	// Loads an image through the app's HttpTransport instead of letting the
	// browser issue an unauthenticated GET, then renders it via a real <img>.
	// Required for proper authentication.
	//
	// This file should be considered a wholesale replacement for <img />.

	import { getPlatformApi } from '$lib/core/context.js';

	let transport = getPlatformApi();

	let { src, alt = '', ...rest } = $props();

	let objectUrl = $state<string | null>(null);
	let status = $state('loading');

	$effect(() => {
		const currentSrc = src;
		let cancelled = false;
		let createdUrl: string | null = null;

		status = 'loading';

		(async () => {
			try {
				const blob = await transport.http.getBlob(currentSrc);
				if (cancelled) return; // a newer src already superseded this fetch
				createdUrl = URL.createObjectURL(blob);
				objectUrl = createdUrl;
				status = 'loaded';
			} catch {
				if (!cancelled) status = 'error';
			}
		})();

		return () => {
			cancelled = true;
			if (createdUrl) URL.revokeObjectURL(createdUrl);
		};
	});
</script>

<span class="transport-image" data-status={status} {...rest}>
	{#if status === 'loaded'}
		<img src={objectUrl} {alt} />
	{:else if status === 'error'}
		<span class="transport-image-error" aria-label={alt}></span>
	{:else}
		<span class="transport-image-loading" aria-label="Loading"></span>
	{/if}
</span>

<style>
	img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.transport-image-loading,
	.transport-image-error {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
