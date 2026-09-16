import type { Snowflake } from '$lib/types/core.js';
import type { RealmResourceKind } from '$lib/types/realm.js';

/**
 * What the asset browser hands to a drop target.
 */
export const ASSET_MIME = 'application/x-lorealm-asset';

export type AssetDrag =
	| { drag: 'resource'; kind: RealmResourceKind; id: Snowflake; name: string }
	| { drag: 'input'; name: string };

/**
 * The drag in flight, readable by every target at once.
 */
let active = $state<AssetDrag | null>(null);

export function activeDrag(): AssetDrag | null {
	return active;
}

export function setDrag(event: DragEvent, payload: AssetDrag) {
	event.dataTransfer?.setData(ASSET_MIME, JSON.stringify(payload));
	event.dataTransfer?.setData('text/plain', payload.drag === 'input' ? payload.name : payload.id);
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

export function carriesAsset(event: DragEvent): boolean {
	return event.dataTransfer?.types.includes(ASSET_MIME) ?? false;
}
