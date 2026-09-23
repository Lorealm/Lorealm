/** The code below is an AI-generated replacement for Luxon.
 * 
 * Generated to save some chunk/bundle size.
 */

function parse(iso: string): Date | null {
	const date = new Date(iso);
	return Number.isNaN(date.getTime()) ? null : date;
}

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
	['year', 365 * 24 * 60 * 60 * 1000],
	['month', 30 * 24 * 60 * 60 * 1000],
	['week', 7 * 24 * 60 * 60 * 1000],
	['day', 24 * 60 * 60 * 1000],
	['hour', 60 * 60 * 1000],
	['minute', 60 * 1000],
	['second', 1000]
];

const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });

export function relativeTime(iso: string, now: number = Date.now()): string | null {
	const date = parse(iso);
	if (!date) return null;

	const delta = date.getTime() - now;
	for (const [unit, ms] of UNITS) {
		if (Math.abs(delta) >= ms) return relative.format(Math.trunc(delta / ms), unit);
	}
	return relative.format(0, 'second');
}

const dateTime = new Intl.DateTimeFormat(undefined, {
	year: 'numeric',
	month: 'short',
	day: 'numeric',
	hour: 'numeric',
	minute: '2-digit'
});

export function dateTimeMedium(iso: string): string | null {
	const date = parse(iso);
	return date ? dateTime.format(date) : null;
}

const monthYearFormat = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long' });

export function monthYear(iso: string): string | null {
	const date = parse(iso);
	return date ? monthYearFormat.format(date) : null;
}

export function daysSince(iso: string, now: number = Date.now()): number {
	const date = parse(iso);
	if (!date) return 0;
	return Math.max(0, Math.floor((now - date.getTime()) / (24 * 60 * 60 * 1000)));
}
