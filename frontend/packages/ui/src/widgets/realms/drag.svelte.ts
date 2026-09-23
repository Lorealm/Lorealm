import type { Snowflake } from '$lib/types/core.js';
import type { RealmResourceKind } from '$lib/types/realm.js';

/**
 * What the asset browser hands to a drop target.
 */
export const ASSET_MIME = 'application/x-lorealm-asset';

export type AssetDrag =
	| { drag: 'resource'; kind: RealmResourceKind; id: Snowflake; name: string }
	/** `path` names a field inside an object input; empty is the input itself. */
	| { drag: 'input'; name: string; path: string[] };

/**
 * The drag in flight, readable by every target at once.
 */
let active = $state<AssetDrag | null>(null);

export function activeDrag(): AssetDrag | null {
	return active;
}

export function inputText(name: string, path: string[]): string {
	return `{{input:${[name, ...path].join('.')}}}`;
}

export function setDrag(event: DragEvent, payload: AssetDrag) {
	event.dataTransfer?.setData(ASSET_MIME, JSON.stringify(payload));
	// CodeMirror inserts `text/plain` at the drop point on its own, so putting
	// the interpolation here is what makes a weaver editor a drop target for
	// free — no handler, no per-editor wiring.
	event.dataTransfer?.setData(
		'text/plain',
		payload.drag === 'input' ? inputText(payload.name, payload.path) : payload.id
	);
	if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
	active = payload;
}

export function clearDrag() {
	active = null;
}

export function readDrag(event: DragEvent): AssetDrag | null {
	const raw = event.dataTransfer?.getData(ASSET_MIME);
	if (!raw) return null;
	try {
		return JSON.parse(raw) as AssetDrag;
	} catch {
		return null;
	}
}

/**
 * The value a plain text field should take after an input is dropped on it
 */
export function dropTextInto(event: DragEvent): string | null {
	const payload = readDrag(event);
	clearDrag();
	if (payload?.drag !== 'input') return null;
	event.preventDefault();

	const field = event.currentTarget as HTMLInputElement;
	const from = field.selectionStart ?? field.value.length;
	const to = field.selectionEnd ?? from;
	return field.value.slice(0, from) + inputText(payload.name, payload.path) + field.value.slice(to);
}

export function carriesAsset(event: DragEvent): boolean {
	return event.dataTransfer?.types.includes(ASSET_MIME) ?? false;
}
