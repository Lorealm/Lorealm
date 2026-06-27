/** A widget instance the model attached to a message. */
export interface WidgetInstance {
	widget_id: string;
	props: Record<string, unknown>;
}

export type ContentSegment = { type: 'text'; text: string } | { type: 'widget'; index: number };

/**
 * Split message content on widget position markers (`\uE000widget:N\uE001`).
 * Widget segments reference indices into the message's `metadata.widgets`.
 */
export function splitWidgetMarkers(content: string): ContentSegment[] {
	const re = /\uE000widget:(\d+)\uE001/g;
	const segments: ContentSegment[] = [];
	let last = 0;
	for (const m of content.matchAll(re)) {
		const at = m.index ?? 0;
		if (at > last) segments.push({ type: 'text', text: content.slice(last, at) });
		segments.push({ type: 'widget', index: Number(m[1]) });
		last = at + m[0].length;
	}
	if (last < content.length || segments.length === 0) {
		segments.push({ type: 'text', text: content.slice(last) });
	}
	return segments;
}

/** Extract the model's reasoning text from a message's metadata, if any. */
export function parseReasoning(metadata: unknown): string {
	if (!metadata || typeof metadata !== 'object') return '';
	const r = (metadata as Record<string, unknown>).reasoning;
	return typeof r === 'string' ? r : '';
}

/** Extract widget instances from a message's metadata, if any. */
export function parseWidgets(metadata: unknown): WidgetInstance[] {
	if (!metadata || typeof metadata !== 'object') return [];
	const widgets = (metadata as Record<string, unknown>).widgets;
	if (!Array.isArray(widgets)) return [];
	return widgets
		.filter(
			(w): w is { widget_id: string; props?: unknown } =>
				!!w && typeof w === 'object' && typeof (w as Record<string, unknown>).widget_id === 'string'
		)
		.map((w) => ({
			widget_id: w.widget_id,
			props: w.props && typeof w.props === 'object' ? (w.props as Record<string, unknown>) : {}
		}));
}
