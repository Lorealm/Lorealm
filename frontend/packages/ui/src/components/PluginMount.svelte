<script lang="ts">
	import { useApi } from '#lib/api/client.js';
	import { pluginCalls } from '#lib/stores/pluginCalls.svelte.js';
	import { pluginEvents } from '#lib/stores/pluginEvents.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	// The SDK documents this API for plugin authors; keep the two in step.
	import type { LorealmApi, PluginModule, SlotContext } from '@lorealm/plugin';

	let {
		handle,
		entry,
		context,
		conversation = null,
		message = null
	}: {
		handle: string;
		entry: string;
		context: SlotContext;
		/** Where the slot is, sent with every call the plugin makes. */
		conversation?: Snowflake | null;
		message?: Snowflake | null;
	} = $props();

	const api = useApi();

	let host = $state<HTMLDivElement | null>(null);
	let failed = $state(false);

	// Plain, not `$state`: the module reads it through `api.context`.
	let current: SlotContext = { conversation: null, message: null };
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const listeners = new Set<(context: SlotContext) => void>();

	$effect(() => {
		current = $state.snapshot(context);
		for (const listener of listeners) listener(current);
	});

	$effect(() => {
		const [h, e, element] = [handle, entry, host];
		if (!element) return;
		failed = false;

		let cancelled = false;
		let unmount: (() => void) | undefined;
		let url: string | undefined;
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const stops = new Set<() => void>();

		const pluginApi: LorealmApi = {
			get context() {
				return current;
			},
			call: async (name, args, options) => {
				// A blocking call holds the chat until the plugin answers.
				const blocking = options?.blocking === true;
				const release = blocking ? pluginCalls.hold(h, conversation) : null;
				try {
					return await api.plugins.call(h, name, args ?? null, {
						conversation,
						message,
						blocking
					});
				} finally {
					release?.();
				}
			},
			on(name, listener) {
				const stop = pluginEvents.on(h, (sent, payload) => {
					if (sent === name) listener(payload);
				});
				stops.add(stop);
				return () => {
					stop();
					stops.delete(stop);
				};
			},
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
			for (const stop of stops) stop();
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
