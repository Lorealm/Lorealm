export type Side = 'top' | 'bottom' | 'left' | 'right';

/** Breathing room kept between a floating panel and the viewport edge. */
const MARGIN = 8;

const OPPOSITE: Record<Side, Side> = { top: 'bottom', bottom: 'top', left: 'right', right: 'left' };

function room(anchor: DOMRect, side: Side): number {
	const { innerWidth: width, innerHeight: height } = window;
	if (side === 'top') return anchor.top - MARGIN;
	if (side === 'bottom') return height - anchor.bottom - MARGIN;
	if (side === 'left') return anchor.left - MARGIN;
	return width - anchor.right - MARGIN;
}

function clamp(value: number, min: number, max: number): number {
	return Math.max(min, Math.min(value, Math.max(min, max)));
}

/**
 * Pins `panel` (a `position: fixed` element) beside `anchor`, moving it to the
 * opposite side when it doesn't fit and keeping it inside the viewport.
 */
export function placeFloating(
	panel: HTMLElement,
	anchor: DOMRect,
	{
		side = 'bottom',
		align = 'start',
		gap = 4,
		bounds
	}: {
		side?: Side;
		align?: 'start' | 'center' | 'end' | 'auto';
		gap?: number;
		/** The box `align: 'auto'` keeps the panel inside; defaults to the viewport. */
		bounds?: DOMRect;
	} = {}
) {
	panel.style.maxHeight = '';
	const { width, height } = panel.getBoundingClientRect();

	const vertical = side === 'top' || side === 'bottom';
	const needed = (vertical ? height : width) + gap;
	const flipped = room(anchor, side) < needed && room(anchor, OPPOSITE[side]) > room(anchor, side);
	const final = flipped ? OPPOSITE[side] : side;

	let left: number;
	let top: number;
	if (vertical) {
		const start =
			align === 'center'
				? anchor.left + (anchor.width - width) / 2
				: align === 'end' ||
					  (align === 'auto' && anchor.left + width > (bounds?.right ?? window.innerWidth))
					? anchor.right - width
					: anchor.left;
		left = clamp(start, MARGIN, window.innerWidth - width - MARGIN);
		const available = room(anchor, final) - gap;
		if (height > available) panel.style.maxHeight = `${Math.max(available, 0)}px`;
		top = final === 'top' ? anchor.top - gap - Math.min(height, available) : anchor.bottom + gap;
	} else {
		left = final === 'left' ? anchor.left - gap - width : anchor.right + gap;
		top = clamp(
			anchor.top + (anchor.height - height) / 2,
			MARGIN,
			window.innerHeight - height - MARGIN
		);
	}

	panel.style.left = `${left}px`;
	panel.style.top = `${top}px`;
}
