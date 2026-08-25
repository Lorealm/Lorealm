/**
 * What the device can do, as far as the keybind manager cares.
 */
class InputEnvironment {
	#coarsePointer = $state(false);
	#hover = $state(true);

	constructor() {
		if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
		this.#track('(pointer: coarse)', (matches) => (this.#coarsePointer = matches));
		this.#track('(hover: hover)', (matches) => (this.#hover = matches));
	}

	#track(query: string, apply: (matches: boolean) => void) {
		const media = window.matchMedia(query);
		apply(media.matches);
		media.addEventListener('change', (event) => apply(event.matches));
	}

	/**
	 * A phone or tablet: a coarse pointer and nothing that hovers. A laptop with
	 * a touchscreen still reports `hover`, so it does not land here.
	 */
	get touchOnly(): boolean {
		return this.#coarsePointer && !this.#hover;
	}
}

export const inputEnvironment = new InputEnvironment();
