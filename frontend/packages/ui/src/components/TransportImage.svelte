<script module lang="ts">
	// Loads an image through the app's HttpTransport instead of letting the
	// browser issue an unauthenticated GET, then renders it via a real <img>.
	// Required for proper authentication.
	//
	// This file should be considered a wholesale replacement for <img />.

	type Entry = {
		/** Resolves to an object URL. Shared by every holder of this src. */
		url: Promise<string>;
		refs: number;
	};

	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const cache = new Map<string, Entry>();

	const idle: string[] = [];

	/**
	 * How many unreferenced images to keep.
	 */
	const RETAINED = 48;

	function acquire(src: string, fetchBlob: (s: string) => Promise<Blob>): Promise<string> {
		let entry = cache.get(src);

		if (!entry) {
			entry = { refs: 0, url: fetchBlob(src).then((blob) => URL.createObjectURL(blob)) };
			cache.set(src, entry);
			// A failure must not be cached: drop it so a later mount can retry.
			entry.url.catch(() => cache.delete(src));
		}

		const resident = idle.indexOf(src);
		if (resident !== -1) idle.splice(resident, 1);

		entry.refs += 1;
		return entry.url;
	}

	function release(src: string) {
		const entry = cache.get(src);
		if (!entry || entry.refs === 0) return;

		entry.refs -= 1;
		if (entry.refs > 0) return;

		idle.push(src);
		while (idle.length > RETAINED) {
			const stale = idle.shift()!;
			const victim = cache.get(stale);
			if (!victim || victim.refs > 0) continue; // re-acquired, or already gone

			cache.delete(stale);
			victim.url.then(URL.revokeObjectURL).catch(() => {});
		}
	}
</script>

<script lang="ts">
	import { getPlatformApi } from '$lib/core/context.js';

	let transport = getPlatformApi();

	let { src, alt = '', ...rest } = $props();

	let objectUrl = $state<string | null>(null);
	let status = $state('loading');

	$effect(() => {
		const currentSrc: string | null = src;
		let cancelled = false;

		if (!currentSrc) {
			objectUrl = null;
			status = 'error';
			return;
		}

		status = 'loading';

		acquire(currentSrc, (s) => transport.http.getBlob(s)).then(
			(url) => {
				if (cancelled) return; // a newer src already superseded this fetch
				objectUrl = url;
				status = 'loaded';
			},
			() => {
				if (!cancelled) status = 'error';
			}
		);

		return () => {
			cancelled = true;
			release(currentSrc);
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
