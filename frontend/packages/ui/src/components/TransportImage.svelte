<script lang="ts">
	// Loads an image through the app's HttpTransport instead of letting the
	// browser issue an unauthenticated GET, then renders it via a real <img>.
	//
	// Usage:
	//   <TransportImage src="/api/v1/characters/abc/image" alt="Aria" class="avatar" />

	// `transport.getBlob(path)` returns a Blob (carries its own content-type).
	// Source `transport` however your package exposes the PlatformAPI
	// (Svelte context, module singleton, or a prop).
	import { getPlatformApi } from '$lib/core/context.js';

	let transport = getPlatformApi();

	let { src, alt = '', ...rest } = $props();

	let objectUrl = $state<string | null>(null);
	let status = $state('loading'); // 'loading' | 'loaded' | 'error'

	$effect(() => {
		const currentSrc = src; // tracked dependency: re-runs when src changes
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

		// Fires on unmount and before every re-run (i.e. whenever src changes).
		return () => {
			cancelled = true;
			if (createdUrl) URL.revokeObjectURL(createdUrl);
		};
	});
</script>

{#snippet error(alt: string | null)}
	<span class="transport-image-error" aria-label={alt}></span>
{/snippet}
{#snippet loading()}
	<span class="transport-image-loading" aria-label="Loading"></span>
{/snippet}

{#if status === 'loaded'}
	<img src={objectUrl} {alt} {...rest} />
{:else if status === 'error'}
	{@render error(alt)}
{:else}
	{@render loading()}
{/if}
