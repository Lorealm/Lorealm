import { useApi } from '#lib/api/client.js';
import { showError } from '#lib/stores/errorToast.svelte.js';
import type { Snowflake } from '#lib/types/core.js';
import type {
	CreateUserWidgetRequest,
	UpdateUserWidgetRequest,
	UserWidget
} from '#lib/types/widgets.js';

/** The user's static display widgets (see [`UserWidget`]). */
class UserWidgetStore {
	get #api() {
		return useApi();
	}

	loadedWidgets = $state<Array<UserWidget>>();

	async loadWidgets() {
		try {
			this.loadedWidgets = await this.#api.userWidgets.list();
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to load widgets' });
		}
	}

	/** Server-shipped builtin widgets, read-only. */
	builtinWidgets = $state<Array<UserWidget>>();

	async loadBuiltins() {
		try {
			this.builtinWidgets = await this.#api.userWidgets.builtins();
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to load widgets' });
		}
	}

	byId(id: Snowflake): UserWidget | undefined {
		return this.loadedWidgets?.find((w) => w.id === id);
	}

	async createWidget(create: CreateUserWidgetRequest): Promise<UserWidget | null> {
		try {
			const widget = await this.#api.userWidgets.create(create);
			this.loadedWidgets = [...(this.loadedWidgets ?? []), widget];
			return widget;
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to create widget' });
			return null;
		}
	}

	async updateWidget(id: Snowflake, update: UpdateUserWidgetRequest): Promise<boolean> {
		try {
			const saved = await this.#api.userWidgets.update(id, update);
			this.loadedWidgets = (this.loadedWidgets ?? []).map((w) => (w.id === id ? saved : w));
			return true;
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to save widget' });
			return false;
		}
	}

	async setEnabled(id: Snowflake, enabled: boolean) {
		const widget = this.byId(id);
		if (!widget) return;
		const previous = widget.enabled;
		widget.enabled = enabled;
		try {
			await this.#api.userWidgets.toggle(id, enabled);
		} catch (e) {
			widget.enabled = previous;
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to toggle widget' });
		}
	}

	async deleteWidget(id: Snowflake) {
		try {
			await this.#api.userWidgets.delete(id);
			this.loadedWidgets = (this.loadedWidgets ?? []).filter((w) => w.id !== id);
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), { title: 'Failed to delete widget' });
		}
	}
}

export const userWidgetStore = new UserWidgetStore();
