import { useApi } from '#lib/api/client.js';
import type { ListedPlugin, ListedVersion } from '#lib/types/plugins.js';

/** Whether version `a` is newer than `b`, going by their numbers. */
export function newer(a: string, b: string): boolean {
	const parts = (v: string) => v.split(/[-+]/)[0].split('.').map(Number);
	const [x, y] = [parts(a), parts(b)];
	for (let i = 0; i < 3; i++) {
		if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
	}
	return false;
}

/** The version a listed plugin is shown as. */
export function shownVersion(plugin: ListedPlugin): ListedVersion {
	return plugin.versions.find((v) => v.version === plugin.offered) ?? plugin.versions[0];
}

class MarketplaceStore {
	plugins = $state<ListedPlugin[]>([]);
	/** The indexes that couldn't be read. */
	unavailable = $state<string[]>([]);
	loaded = $state(false);
	failed = $state(false);

	/** In-flight fetch, so concurrent callers share one request. */
	private pending: Promise<void> | null = null;

	find(handle: string): ListedPlugin | undefined {
		return this.plugins.find((p) => p.handle === handle);
	}

	/**
	 * Check whether there is an update available for the specified
	 * plugin.
	 */
	updateFor(handle: string): string | null {
		const listed = this.find(handle);
		if (!listed?.offered || !listed.installed) return null;
		return newer(listed.offered, listed.installed) ? listed.offered : null;
	}

	reload(): Promise<void> {
		this.loaded = false;
		return this.load();
	}

	async load(): Promise<void> {
		if (this.loaded) return;
		if (this.pending) return this.pending;

		this.pending = useApi()
			.plugins.marketplace()
			.then((marketplace) => {
				this.plugins = marketplace.plugins.filter((p) => p.versions.length > 0);
				this.unavailable = marketplace.unavailable;
				this.failed = false;
			})
			.catch(() => {
				this.failed = true;
			})
			.finally(() => {
				this.loaded = true;
				this.pending = null;
			});

		return this.pending;
	}
}

export const marketplaceStore = new MarketplaceStore();
