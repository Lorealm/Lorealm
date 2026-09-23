<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import type { JsonValue } from '$lib/types/core.js';

	/** What an unsafe slot's module is handed; the same as a frame's `lorealm`. */
	interface PluginApi {
		readonly context: JsonValue;
		call(name: string, args?: JsonValue): Promise<JsonValue>;
		onContext(listener: (context: JsonValue) => void): () => void;
	}

	interface PluginModule {
		mount(element: HTMLElement, api: PluginApi): void | (() => void);
	}

	let {
		handle,
		entry,
		context
	}: {
		handle: string;
		entry: string;
		context: JsonValue;
	} = $props();

	const api = useApi();

	let host = $state<HTMLDivElement | null>(null);
	let failed = $state(false);

	// Plain, not `$state`: the module reads it through `api.context`.
	let current: JsonValue = null;
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const listeners = new Set<(context: JsonValue) => void>();

	$effect(() => {
		current = $state.snapshot(context as unknown) as JsonValue;
		for (const listener of listeners) listener(current);
	});

	$effect(() => {
		const [h, e, element] = [handle, entry, host];
		if (!element) return;
		failed = false;

		let cancelled = false;
		let unmount: (() => void) | undefined;
		let url: string | undefined;

		const pluginApi: PluginApi = {
			get context() {
				return current;
			},
			call: (name, args) => api.plugins.call(h, name, args ?? null),
			onContext(listener) {
				listeners.add(listener);
				return () => listeners.delete(listener);
			}
		};

		void (async () => {
			try {
				const code = await api.plugins.frontendEntry(h, e);
				if (cancelled) return;
				url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
				const module = (await import(/* @vite-ignore */ url)) as PluginModule;
				if (cancelled) return;
				unmount = module.mount(element, pluginApi) ?? undefined;
			} catch (err) {
				console.error(`plugin ${h}: ${e} failed to mount`, err);
				failed = true;
			}
		})();

		return () => {
			cancelled = true;
			unmount?.();
			listeners.clear();
			if (url) URL.revokeObjectURL(url);
		};
	});
</script>

{#if failed}
	<div class="empty">This plugin's panel couldn't be loaded.</div>
{/if}
<div class="mount" bind:this={host}></div>

<style>
	.mount {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.empty {
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
