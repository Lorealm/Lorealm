<script lang="ts">
	import type { JsonValue } from '#lib/types/core.js';
	import type { PlayerAction } from '#lib/types/realm.js';
	import { frameTheme } from '#lib/stores/frameTheme.svelte.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { macroContext } from '#lib/assets/scripts/variable-replace.js';

	let {
		source,
		slice,
		title,
		kind = 'state',
		onaction
	}: {
		source: string;
		slice: Record<string, JsonValue>;
		title: string;
		kind?: 'state' | 'props';
		/** A tool the widget triggers, or Weaver it sends, and what to call it. */
		onaction?: (action: PlayerAction, label: string) => void;
	} = $props();

	/**
	 * Injected at the end of every widget. Provides functionality for:
	 * - Auto-resizing
	 * - Variable access
	 * - Automatic theme-adherence & CSS variables
	 * - Ready handling
	 */
	const HOST_SCRIPT = `<script>(function () {
  function report() {
    var b = document.body;
    if (!b) return;
    var cs = getComputedStyle(b);
    var h = Math.ceil(
      b.getBoundingClientRect().height +
      parseFloat(cs.marginTop || '0') +
      parseFloat(cs.marginBottom || '0')
    );
    parent.postMessage({ type: 'lorealm:height', height: h }, '*');
  }
  function ready() { parent.postMessage({ type: 'lorealm:ready' }, '*'); }
  var faces = [];
  var written = new WeakMap();
  function expand(text, ctx) {
    return text.replace(/\\{\\{(\\w+):(\\w+)\\}\\}/g, function (m, who, field) {
      var side = ctx[who];
      var v = side && side[field];
      return typeof v === 'string' ? v : m;
    });
  }
  function fill(ctx) {
    window.lorealm = window.lorealm || {};
    window.lorealm.context = ctx;
    var root = document.body || document.documentElement;
    var walk = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    for (var n = walk.nextNode(); n; n = walk.nextNode()) {
      var tag = n.parentNode && n.parentNode.nodeName;
      if (tag === 'SCRIPT' || tag === 'STYLE') continue;
      var seen = written.get(n);
      // A widget script that rewrote the node since we last filled it owns the new text.
      var tpl = seen && seen.out === n.nodeValue ? seen.tpl : n.nodeValue;
      if (tpl.indexOf('{{') < 0) continue;
      var out = expand(tpl, ctx);
      written.set(n, { tpl: tpl, out: out });
      if (out !== n.nodeValue) n.nodeValue = out;
    }
  }
  addEventListener('message', function (e) {
    var d = e.data;
    if (e.source !== parent || !d) return;
    if (d.type === 'lorealm:context') {
      fill(d.context || {});
    } else if (d.type === 'lorealm:theme') {
      var style = document.getElementById('lorealm-theme');
      if (!style) {
        style = document.createElement('style');
        style.id = 'lorealm-theme';
        document.head.prepend(style);
      }
      style.textContent = d.css;
    } else if (d.type === 'lorealm:fonts') {
      faces.forEach(function (f) { document.fonts.delete(f); });
      faces = d.fonts.map(function (f) {
        var face = new FontFace(f.family, f.buffer, f.descriptors);
        document.fonts.add(face);
        return face;
      });
    }
  });
  try { new ResizeObserver(report).observe(document.body); } catch (e) {}
  addEventListener('load', function () { report(); ready(); });
  report();
  ready();
})();${'</'}script>`;

	let frame = $state<HTMLIFrameElement | null>(null);
	let height = $state(0);

	function push() {
		const payload = $state.snapshot(slice);
		frame?.contentWindow?.postMessage(
			// `state` stays on the payload whatever the kind, so a widget
			// author's `e.data.state` keeps working.
			{ type: `lorealm:${kind}`, state: payload, props: payload },
			'*'
		);
	}

	function pushTheme() {
		frame?.contentWindow?.postMessage({ type: 'lorealm:theme', css: frameTheme.css }, '*');
		frame?.contentWindow?.postMessage({ type: 'lorealm:fonts', fonts: frameTheme.fonts }, '*');
	}

	/** The same names `{{char:name}}` and `{{user:name}}` resolve to in messages. */
	const context = $derived(macroContext(characterStore.activeCharacter?.id ?? null));

	function pushContext() {
		frame?.contentWindow?.postMessage({ type: 'lorealm:context', context }, '*');
	}

	$effect(() => {
		void slice;
		push();
	});

	$effect(() => {
		void context;
		pushContext();
	});

	$effect(() => {
		void frameTheme.css;
		void frameTheme.fonts;
		pushTheme();
	});

	function onMessage(e: MessageEvent) {
		if (!frame || e.source !== frame.contentWindow) return;
		const data = e.data as {
			type?: string;
			tool_id?: unknown;
			args?: unknown;
			source?: unknown;
			label?: unknown;
			height?: unknown;
		} | null;
		if (data?.type === 'lorealm:ready') {
			pushContext();
			push();
			pushTheme();
		} else if (data?.type === 'lorealm:height' && typeof data.height === 'number') {
			height = Math.max(0, Math.min(4000, data.height));
		} else if (data?.type === 'lorealm:action' && typeof data.tool_id === 'string') {
			onaction?.({ tool_id: data.tool_id, args: (data.args ?? {}) as JsonValue }, labelOf(data));
		} else if (data?.type === 'lorealm:run' && typeof data.source === 'string') {
			onaction?.({ run: data.source }, labelOf(data));
		}
	}

	function labelOf(data: { label?: unknown }): string {
		return typeof data.label === 'string' && data.label.trim() ? data.label : title;
	}
</script>

<svelte:window onmessage={onMessage} />

<iframe
	class="static-widget"
	{title}
	sandbox="allow-scripts"
	srcdoc={source + HOST_SCRIPT}
	bind:this={frame}
	onload={push}
	style:height="{height}px"
></iframe>

<style>
	/* Bare: the widget's own HTML decides how it looks. */
	.static-widget {
		display: block;
		width: 100%;
		border: none;
		background: transparent;
	}
</style>
