<script lang="ts">
	import type { JsonValue } from '$lib/types/core.js';

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
		onaction?: (toolId: string, args: JsonValue) => void;
	} = $props();

	/**
	 * Injected at the end of every widget so authors don't hand-write it:
	 * reports content height (iframes can't self-size) and pings ready so the
	 * host pushes the initial payload.
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

	$effect(() => {
		void slice;
		push();
	});

	function onMessage(e: MessageEvent) {
		if (!frame || e.source !== frame.contentWindow) return;
		const data = e.data as {
			type?: string;
			tool_id?: unknown;
			args?: unknown;
			height?: unknown;
		} | null;
		if (data?.type === 'lorealm:ready') {
			push();
		} else if (data?.type === 'lorealm:height' && typeof data.height === 'number') {
			height = Math.max(0, Math.min(4000, data.height));
		} else if (data?.type === 'lorealm:action' && typeof data.tool_id === 'string') {
			onaction?.(data.tool_id, (data.args ?? {}) as JsonValue);
		}
	}
</script>

<svelte:window onmessage={onMessage} />

<iframe
	class="static-widget"
	class:bare={kind === 'props'}
	{title}
	sandbox="allow-scripts"
	srcdoc={source + HOST_SCRIPT}
	bind:this={frame}
	onload={push}
	style:height="{height}px"
></iframe>

<style>
	.static-widget.bare {
		border: none;
		border-radius: 0;
		background: transparent;
	}
	.static-widget {
		display: block;
		width: 100%;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		background: var(--color-bg-elevated);
	}
</style>
