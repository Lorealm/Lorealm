export const NARROW_QUERY = '(max-width: 768px)';

class Shell {
	narrow = $state(false);
	#drawer = $state<'nav' | 'inspector' | null>(null);

	constructor() {
		if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
		const media = window.matchMedia(NARROW_QUERY);
		this.narrow = media.matches;
		media.addEventListener('change', (event) => {
			this.narrow = event.matches;
			this.#drawer = null;
		});
	}

	get navOpen(): boolean {
		return this.narrow && this.#drawer === 'nav';
	}
	get inspectorOpen(): boolean {
		return this.narrow && this.#drawer === 'inspector';
	}
	get drawerOpen(): boolean {
		return this.narrow && this.#drawer !== null;
	}

	openNav() {
		this.#drawer = 'nav';
	}
	openInspector() {
		this.#drawer = 'inspector';
	}
	closeDrawer() {
		this.#drawer = null;
	}
}

export const shell = new Shell();
