import type { UpdateControls } from '#lib/core/api.js';
import { dismissToast, showError, showToast } from './errorToast.svelte.ts';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * Asks the platform for a newer release and offers it in a toast.
 */
export async function offerUpdate(updates: UpdateControls | undefined) {
	if (!updates) return;

	let update;
	try {
		update = await updates.check();
	} catch (e) {
		console.warn('[UPDATE] Check failed: ' + JSON.stringify(e));
		return;
	}
	if (!update) return;

	const install = async () => {
		const progress = showToast({
			title: t.shell.update.installingTitle,
			message: t.shell.update.installing,
			durationMs: 0
		});
		try {
			await update.install();
		} catch (e) {
			dismissToast(progress);
			showError(e, { title: t.shell.update.failed });
		}
	};

	showToast({
		title: t.shell.update.title,
		message: t.shell.update.message({ version: update.version }),
		durationMs: 0,
		actions: [
			{ label: t.shell.update.install, tone: 'primary', run: () => void install() },
			{ label: t.shell.update.later, run: () => {} }
		]
	});
}
