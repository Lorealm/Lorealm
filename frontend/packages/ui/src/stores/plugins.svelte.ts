import { useApi } from '#lib/api/client.js';
import type { FrontendSlot, InstalledPlugin } from '#lib/types/plugins.js';
import { pluginWidgetKey, type PluginWidgetKey } from '#lib/widgets/inspector/registry.js';

/** A slot one of the user's enabled plugins fills. */
export interface PluginSlot extends FrontendSlot {
	key: PluginWidgetKey;
	handle: string;
	/** May play media without the user clicking in it first. */
	autoplay: boolean;
}

class PluginsStore {
	plugins = $state<InstalledPlugin[]>([]);
	loaded = $state(false);

	/** In-flight fetch, so concurrent callers share one request. */
	private pending: Promise<void> | null = null;

	get #api() {
		return useApi();
	}

	/** Every `kind` slot the user's enabled plugins fill. */
	slots(kind: string): PluginSlot[] {
		return this.plugins
			.filter((p) => p.enabled)
			.flatMap((p) =>
				p.slots
					.filter((s) => s.slot === kind)
					.map((s) => ({
						...s,
						key: pluginWidgetKey(p.handle, s.entry),
						handle: p.handle,
						autoplay: p.permissions.includes('ui:autoplay')
					}))
			);
	}

	/** The inspector slot behind `key`, if its plugin is still enabled. */
	inspectorSlot(key: PluginWidgetKey): PluginSlot | undefined {
		return this.slots('inspector').find((s) => s.key === key);
	}

	/** Re-fetch the list */
	reload(): Promise<void> {
		this.loaded = false;
		return this.load();
	}

	/** Fetch the list once. Repeat calls reuse the first result. */
	async load(): Promise<void> {
		if (this.loaded) return;
		if (this.pending) return this.pending;

		this.pending = this.#api.plugins
			.list()
			.then((plugins) => {
				this.plugins = plugins;
				this.loaded = true;
			})
			.finally(() => {
				this.pending = null;
			});

		return this.pending;
	}
}

export const pluginsStore = new PluginsStore();
