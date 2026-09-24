<script lang="ts">
	import { untrack } from 'svelte';
	import { useApi } from '$lib/api/client.js';
	import { frameTheme } from '$lib/stores/frameTheme.svelte.js';
	import type { JsonValue } from '$lib/types/core.js';

	let {
		handle,
		entry,
		title,
		context,
		fill = false,
		autoplay = false
	}: {
		handle: string;
		entry: string;
		title: string;
		/** What the slot tells the plugin, such as the message it's attached to. */
		context: JsonValue;
		/** Fill the parent, instead of growing to fit the content. */
		fill?: boolean;
		/** Lets it play media without a click first; the `ui:autoplay` permission. */
		autoplay?: boolean;
	} = $props();

	const api = useApi();

	/**
	 * No network, and nothing loaded from outside the entry itself: a plugin
	 * frame can only reach the world through `lorealm.call`, which goes to the
	 * plugin's own backend. `connect-src` falls back to `default-src`.
	 */
	const CSP =
		"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; " +
		'img-src data: blob:; font-src data:; media-src data: blob:';

	/**
	 * Gives the frame `lorealm` (`call`, `context`, `onContext`) before any
	 * plugin script runs, applies theme and font updates, and reports its height.
	 */
	function bridge(context: JsonValue): string {
		// JSON is valid JS; escaping `<` keeps it from ending the script early.
		const initial = JSON.stringify(context).replace(/</g, '\\u003c');
		return `<script>(function () {
  var pending = {}, next = 0, listeners = [], faces = [];
  window.lorealm = {
    context: ${initial},
    call: function (name, args) {
      return new Promise(function (resolve, reject) {
        var id = ++next;
        pending[id] = { resolve: resolve, reject: reject };
        parent.postMessage({ type: 'lorealm:call', id: id, name: name, args: args === undefined ? null : args }, '*');
      });
    },
    onContext: function (listener) {
      listeners.push(listener);
      return function () { listeners = listeners.filter(function (l) { return l !== listener; }); };
    }
  };
  addEventListener('message', function (e) {
    var d = e.data;
    if (e.source !== parent || !d) return;
    if (d.type === 'lorealm:result' && pending[d.id]) {
      var p = pending[d.id];
      delete pending[d.id];
      if (d.ok) p.resolve(d.value); else p.reject(new Error(d.error));
    } else if (d.type === 'lorealm:context') {
      lorealm.context = d.context;
      listeners.forEach(function (l) { l(d.context); });
    } else if (d.type === 'lorealm:theme') {
      document.getElementById('lorealm-theme').textContent = d.css;
    } else if (d.type === 'lorealm:fonts') {
      faces.forEach(function (f) { document.fonts.delete(f); });
      faces = d.fonts.map(function (f) {
        var face = new FontFace(f.family, f.buffer, f.descriptors);
        document.fonts.add(face);
        return face;
      });
    }
  });
  function report() {
    var b = document.body;
    if (!b) return;
    var cs = getComputedStyle(b);
    var h = Math.ceil(b.getBoundingClientRect().height + parseFloat(cs.marginTop || '0') + parseFloat(cs.marginBottom || '0'));
    parent.postMessage({ type: 'lorealm:height', height: h }, '*');
  }
  addEventListener('load', function () {
    report();
    try { new ResizeObserver(report).observe(document.body); } catch (e) {}
  });
})();${'</'}script>`;
	}

	/** A plain copy of the context: a state proxy can't be posted to a frame. */
	function snapshot(): JsonValue {
		return $state.snapshot(context as unknown) as JsonValue;
	}

	let html = $state<string | null>(null);
	let failed = $state(false);
	let loaded = $state(false);
	let height = $state(0);
	let frame = $state<HTMLIFrameElement | null>(null);

	$effect(() => {
		const [h, e] = [handle, entry];
		html = null;
		failed = false;
		loaded = false;
		api.plugins.frontendEntry(h, e).then(
			(text) => (html = text),
			() => (failed = true)
		);
	});

	// The CSP must come first
	const srcdoc = $derived.by(() => {
		if (html === null) return null;
		const [initialContext, initialTheme] = untrack(() => [snapshot(), frameTheme.css] as const);
		return (
			`<!doctype html><meta http-equiv="Content-Security-Policy" content="${CSP}">` +
			`<style id="lorealm-theme">${initialTheme}</style>${bridge(initialContext)}` +
			html.replace(/^\s*<!doctype[^>]*>/i, '')
		);
	});

	function post(message: Record<string, unknown>) {
		frame?.contentWindow?.postMessage(message, '*');
	}

	/** Catches up on anything that changed while the page was loading. */
	function onload() {
		loaded = true;
		post({ type: 'lorealm:context', context: snapshot() });
		post({ type: 'lorealm:theme', css: frameTheme.css });
		post({ type: 'lorealm:fonts', fonts: frameTheme.fonts });
	}

	$effect(() => {
		const current = snapshot();
		if (untrack(() => loaded)) post({ type: 'lorealm:context', context: current });
	});

	$effect(() => {
		const css = frameTheme.css;
		if (untrack(() => loaded)) post({ type: 'lorealm:theme', css });
	});

	$effect(() => {
		const fonts = frameTheme.fonts;
		if (untrack(() => loaded)) post({ type: 'lorealm:fonts', fonts });
	});

	function onMessage(e: MessageEvent) {
		if (!frame || e.source !== frame.contentWindow) return;
		const data = e.data as {
			type?: unknown;
			id?: unknown;
			name?: unknown;
			args?: unknown;
			height?: unknown;
		} | null;
		if (data?.type === 'lorealm:height' && typeof data.height === 'number') {
			height = Math.max(0, Math.min(4000, data.height));
			return;
		}
		if (
			data?.type !== 'lorealm:call' ||
			typeof data.id !== 'number' ||
			typeof data.name !== 'string'
		)
			return;
		const id = data.id;
		api.plugins.call(handle, data.name, (data.args ?? null) as JsonValue).then(
			(value) => post({ type: 'lorealm:result', id, ok: true, value }),
			(err: unknown) =>
				post({
					type: 'lorealm:result',
					id,
					ok: false,
					error: err instanceof Error ? err.message : 'The call failed.'
				})
		);
	}
</script>

<svelte:window onmessage={onMessage} />

{#if failed}
	<div class="empty">This plugin's panel couldn't be loaded.</div>
{:else if srcdoc !== null}
	<iframe
		{title}
		class:fill
		sandbox="allow-scripts"
		allow={autoplay ? 'autoplay' : undefined}
		{srcdoc}
		bind:this={frame}
		{onload}
		style:height={fill ? undefined : `${height}px`}
	></iframe>
{/if}

<style>
	iframe {
		display: block;
		width: 100%;
		border: none;
		background: transparent;
	}
	iframe.fill {
		flex: 1;
		min-height: 0;
	}
	.empty {
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
