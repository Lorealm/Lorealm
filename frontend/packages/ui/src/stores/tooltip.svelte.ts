import type { Snippet } from 'svelte';
import type { Attachment } from 'svelte/attachments';
import type { Side } from '../components/floating.ts';

export interface TooltipOptions {
	/** Plain text, or a snippet for anything richer — icons, buttons, lists. */
	content: string | Snippet;
	/** What assistive tech announces when `content` is a snippet. */
	label?: string;
	side?: Side;
	/** Keeps the tooltip open under the pointer and reachable by Tab, for content with controls. */
	interactive?: boolean;
	/** Milliseconds of hover before it appears. */
	delay?: number;
}

export type TooltipInput = string | Snippet | TooltipOptions | null | undefined;

const HOVER_DELAY = 400;
/** Moving between neighbouring controls within this window skips the hover delay. */
const WARM_WINDOW = 300;
/** Lets the pointer cross the gap from the anchor onto an interactive tooltip. */
const HOVER_BRIDGE = 150;
const FOCUSABLE = 'a[href], button:not(:disabled), input:not(:disabled), select, textarea';

const anchors = new WeakMap<Element, TooltipOptions>();

export const tooltipState = $state<{
	anchor: HTMLElement | SVGElement | null;
	options: TooltipOptions | null;
	panel: HTMLElement | null;
}>({ anchor: null, options: null, panel: null });

let source: 'hover' | 'focus' = 'hover';
let pending: Element | null = null;
let showTimer: ReturnType<typeof setTimeout> | undefined;
let hideTimer: ReturnType<typeof setTimeout> | undefined;
let lastHidden = 0;
/** The anchor just pressed: it stays quiet until the pointer leaves it. */
let quieted: Element | null = null;

function normalize(input: TooltipInput): TooltipOptions | null {
	if (!input) return null;
	return typeof input === 'object' ? input : { content: input };
}

function findAnchor(from: Element | null): HTMLElement | SVGElement | null {
	for (let el = from; el; el = el.parentElement) {
		if (anchors.has(el)) return el as HTMLElement | SVGElement;
	}
	return null;
}

/** A tooltip replaces `title`, which also named icon-only controls for screen readers. */
function announce(el: Element, text: string | undefined) {
	const visible = el.textContent?.trim();
	if (!text || text === visible) return undefined;
	const named = el.hasAttribute('aria-label') || !!visible;
	const attr = named ? 'aria-description' : 'aria-label';
	if (el.getAttribute(attr) === text || el.getAttribute('aria-label') === text) return undefined;
	el.setAttribute(attr, text);
	return () => el.removeAttribute(attr);
}

/**
 * `{@attach tooltip('Delete')}` on any element, or pass options:
 * `{@attach tooltip({ content: details, label: 'Details', interactive: true })}`.
 * An empty value shows nothing.
 */
export function tooltip(input: TooltipInput): Attachment {
	const options = normalize(input);
	return (el) => {
		if (!options) return;
		anchors.set(el, options);
		const text = typeof options.content === 'string' ? options.content : options.label;
		const undoAnnounce = announce(el, text);
		if (tooltipState.anchor === el) tooltipState.options = options;
		return () => {
			anchors.delete(el);
			undoAnnounce?.();
			// A reactive re-run detaches and reattaches in one go: only an anchor
			// that stays detached has really gone away.
			queueMicrotask(() => {
				if (tooltipState.anchor === el && !anchors.has(el)) hideTooltip();
				if (pending === el && !anchors.has(el)) clearPending();
			});
		};
	};
}

function clearPending() {
	clearTimeout(showTimer);
	pending = null;
}

function show(anchor: HTMLElement | SVGElement, via: 'hover' | 'focus') {
	clearPending();
	clearTimeout(hideTimer);
	source = via;
	tooltipState.anchor = anchor;
	tooltipState.options = anchors.get(anchor) ?? null;
}

function showAfterDelay(anchor: HTMLElement | SVGElement) {
	if (pending === anchor) return;
	clearPending();
	const warm = tooltipState.anchor !== null || performance.now() - lastHidden < WARM_WINDOW;
	if (tooltipState.anchor) hideTooltip();
	const delay = anchors.get(anchor)?.delay ?? HOVER_DELAY;
	if (warm) return show(anchor, 'hover');
	pending = anchor;
	showTimer = setTimeout(() => show(anchor, 'hover'), delay);
}

/** Closes whatever tooltip is showing — for a button inside one that has done its job. */
export function hideTooltip() {
	clearPending();
	clearTimeout(hideTimer);
	if (tooltipState.anchor) lastHidden = performance.now();
	tooltipState.anchor = null;
	tooltipState.options = null;
}

function focusInPanel() {
	return !!tooltipState.panel?.contains(document.activeElement);
}

function scheduleHide() {
	if (!tooltipState.anchor || source === 'focus' || focusInPanel()) return;
	if (!tooltipState.options?.interactive) return hideTooltip();
	clearTimeout(hideTimer);
	hideTimer = setTimeout(hideTooltip, HOVER_BRIDGE);
}

/**
 * Watches the pointer at the window and hit-tests rather than listening on each
 * anchor: disabled buttons, which often carry the reason they're disabled, don't
 * reliably receive pointer events of their own.
 */
export function installTooltips(): () => void {
	let x = 0;
	let y = 0;
	let frame = 0;

	function update() {
		frame = 0;
		const hit = document.elementFromPoint(x, y);
		if (hit && tooltipState.panel?.contains(hit)) {
			clearTimeout(hideTimer);
			return;
		}
		const next = findAnchor(hit);
		if (next !== quieted) quieted = null;
		if (!next) {
			clearPending();
			scheduleHide();
		} else if (next === tooltipState.anchor) {
			clearTimeout(hideTimer);
		} else if (next !== quieted) {
			showAfterDelay(next);
		}
	}

	function onPointerMove(e: PointerEvent) {
		if (e.pointerType === 'touch') return;
		x = e.clientX;
		y = e.clientY;
		frame ||= requestAnimationFrame(update);
	}

	function onPointerLeave() {
		clearPending();
		quieted = null;
		scheduleHide();
	}

	function onPointerDown(e: PointerEvent) {
		const target = e.target as Element;
		if (tooltipState.panel?.contains(target)) return;
		quieted = findAnchor(target);
		hideTooltip();
	}

	function onFocusIn(e: FocusEvent) {
		const target = e.target as Element;
		if (tooltipState.panel?.contains(target)) return clearTimeout(hideTimer);
		const anchor = findAnchor(target);
		if (anchor && anchor !== quieted && target.matches(':focus-visible')) show(anchor, 'focus');
	}

	function onFocusOut(e: FocusEvent) {
		const moved = e.relatedTarget as Node | null;
		if (tooltipState.panel?.contains(moved) || tooltipState.anchor?.contains(moved)) return;
		const leftPanel = tooltipState.panel?.contains(e.target as Node);
		if (source !== 'focus' && !leftPanel) return;
		// Removing a focused element fires this synchronously, mid-render, where
		// Svelte forbids touching state. By the microtask, focus may have landed on
		// another anchor, whose tooltip must survive.
		const leaving = tooltipState.anchor;
		queueMicrotask(() => {
			if (tooltipState.anchor === leaving) hideTooltip();
		});
	}

	function focusAnchor() {
		(tooltipState.anchor as HTMLElement | null)?.focus();
	}

	// The panel sits at the end of the document, so Tab has to be walked in and out of it.
	function onTab(e: KeyboardEvent) {
		const { anchor, options, panel } = tooltipState;
		if (!anchor || !panel || !options?.interactive) return;
		const stops = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
		if (!stops.length) return;
		const active = document.activeElement;
		if (anchor.contains(active) && !e.shiftKey) {
			e.preventDefault();
			stops[0].focus();
		} else if (active === stops[0] && e.shiftKey) {
			e.preventDefault();
			focusAnchor();
		} else if (active === stops.at(-1) && !e.shiftKey) {
			// Not prevented: Tab carries on from the anchor, out past the tooltip.
			focusAnchor();
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === 'Tab') return onTab(e);
		if (e.key !== 'Escape' || !tooltipState.anchor) return;
		// Escape inside a tooltip belongs to it. Otherwise it passes on, so a
		// tooltip left over a modal's button doesn't swallow closing the modal.
		if (focusInPanel()) {
			e.preventDefault();
			e.stopImmediatePropagation();
			focusAnchor();
		}
		hideTooltip();
	}

	function onScroll(e: Event) {
		if (tooltipState.panel?.contains(e.target as Node)) return;
		hideTooltip();
	}

	const capture = { capture: true };
	const passive = { capture: true, passive: true };
	const root = document.documentElement;
	window.addEventListener('pointermove', onPointerMove, passive);
	root.addEventListener('pointerleave', onPointerLeave);
	window.addEventListener('pointerdown', onPointerDown, capture);
	window.addEventListener('focusin', onFocusIn, capture);
	window.addEventListener('focusout', onFocusOut, capture);
	window.addEventListener('keydown', onKeyDown, capture);
	window.addEventListener('scroll', onScroll, passive);
	window.addEventListener('resize', hideTooltip);
	window.addEventListener('blur', hideTooltip);

	return () => {
		cancelAnimationFrame(frame);
		hideTooltip();
		window.removeEventListener('pointermove', onPointerMove, passive);
		root.removeEventListener('pointerleave', onPointerLeave);
		window.removeEventListener('pointerdown', onPointerDown, capture);
		window.removeEventListener('focusin', onFocusIn, capture);
		window.removeEventListener('focusout', onFocusOut, capture);
		window.removeEventListener('keydown', onKeyDown, capture);
		window.removeEventListener('scroll', onScroll, passive);
		window.removeEventListener('resize', hideTooltip);
		window.removeEventListener('blur', hideTooltip);
	};
}
