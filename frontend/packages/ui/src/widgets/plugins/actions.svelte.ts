import { useApi } from '#lib/api/client.js';
import { configStore } from '#lib/stores/serverConfig.svelte.js';
import { marketplaceStore } from '#lib/stores/marketplace.svelte.js';
import { pluginsStore } from '#lib/stores/plugins.svelte.js';
import { Permission } from '#lib/types/permissions.js';

export function canInstall(): boolean {
	return configStore.can(Permission.INSTALL_PLUGINS) || configStore.can(Permission.BUILD_PLUGINS);
}

export async function change(change: () => Promise<unknown>): Promise<string> {
	let error = '';
	try {
		await change();
	} catch (e) {
		error = e instanceof Error ? e.message : String(e);
	}
	await Promise.all([pluginsStore.reload(), marketplaceStore.reload()]);
	return error;
}

/**
 * Installs the marketplace's version of `handle` on the server.
 */
export async function installListed(handle: string): Promise<void> {
	const api = useApi();
	const mine = pluginsStore.plugins.find((p) => p.handle === handle);
	await api.plugins.installListed(handle);
	if (mine?.enabled) await api.plugins.update(handle);
	else await api.plugins.toggle(handle, true);
}
