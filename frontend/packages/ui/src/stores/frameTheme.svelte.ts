import { loadedFonts, rootVariables, type FrameFont } from '$lib/core/frameStyle.js';

class FrameTheme {
	css = $state('');
	fonts = $state.raw<FrameFont[]>([]);

	#watching = false;
	#latest = 0;

	/** Re-read after the app's style changes. */
	refresh(): void {
		this.css = rootVariables();
		void this.#refreshFonts();
		if (!this.#watching) {
			// Faces load as text needs them, after the style is applied.
			this.#watching = true;
			document.fonts.addEventListener('loadingdone', () => void this.#refreshFonts());
		}
	}

	async #refreshFonts(): Promise<void> {
		const call = ++this.#latest;
		const fonts = await loadedFonts();
		const key = (list: FrameFont[]) => list.map((f) => f.url).join('\n');
		if (call === this.#latest && key(fonts) !== key(this.fonts)) this.fonts = fonts;
	}
}

export const frameTheme = new FrameTheme();
