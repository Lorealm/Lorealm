import { t } from '#lib/stores/i18n.svelte.js';

/**
 * The max length of the output string-represented image
 */
const BUDGET = 800_000;

/**
 * Longest edges to try, and the quality ladder tried at each.
 *
 * Resolutions >1080p are not yet supported and will require either
 * server-side storage, or storing/accessing as a file.
 */
const EDGES = [1920, 1344, 960];
const QUALITIES = [0.82, 0.7, 0.55];

function load(dataUri: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const element = new Image();
		element.onload = () => resolve(element);
		element.onerror = () => reject(new Error(t.errors.failed.imageUnreadable));
		element.src = dataUri;
	});
}

function draw(image: HTMLImageElement, edge: number): HTMLCanvasElement | null {
	const scale = Math.min(1, edge / Math.max(image.width, image.height));
	const canvas = document.createElement('canvas');
	canvas.width = Math.max(1, Math.round(image.width * scale));
	canvas.height = Math.max(1, Math.round(image.height * scale));

	const context = canvas.getContext('2d');
	if (!context) return null;

	context.drawImage(image, 0, 0, canvas.width, canvas.height);
	return canvas;
}

/**
 * Shrink a picked image until it is worth storing, and return it as a `data:`
 * URI again.
 */
export async function fitImageForStorage(dataUri: string): Promise<string> {
	const image = await load(dataUri);
	if (image.width === 0 || image.height === 0) {
		throw new Error(t.errors.failed.imageEmpty);
	}

	let type = 'image/webp';
	let best: string | null = null;

	for (const edge of EDGES) {
		const canvas = draw(image, edge);
		if (!canvas) return dataUri;

		for (const quality of QUALITIES) {
			let candidate = canvas.toDataURL(type, quality);
			if (!candidate.startsWith(`data:${type}`)) {
				type = 'image/jpeg';
				candidate = canvas.toDataURL(type, quality);
			}

			if (best === null || candidate.length < best.length) best = candidate;
			if (candidate.length <= BUDGET) return shorter(best, dataUri);
		}
	}

	return shorter(best ?? dataUri, dataUri);
}

function shorter(a: string, b: string): string {
	return a.length < b.length ? a : b;
}
