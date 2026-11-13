import { useApi } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';
import type { User, UserSession } from '#lib/types/user.js';

/**
 * The signed-in user's own profile and devices.
 */
class CurrentUserStore {
	user = $state<User | null>(null);
	sessions = $state<Array<UserSession>>([]);

	get #api() {
		return useApi();
	}

	/** The user's effective name. */
	get name(): string {
		return this.user?.display_name || this.user?.username || '';
	}

	async load() {
		this.user = await this.#api.users.me();
	}

	async loadSessions() {
		this.sessions = await this.#api.users.sessions.list();
	}

	async setDisplayName(displayName: string) {
		this.user = await this.#api.users.updateProfile({ display_name: displayName });
	}

	async setAvatar(avatar: Blob) {
		const { avatar_uri } = await this.#api.users.setAvatar(avatar);
		if (this.user) this.user = { ...this.user, avatar_uri };
	}

	async revokeSession(id: Snowflake) {
		await this.#api.users.sessions.revoke(id);
		this.sessions = this.sessions.filter((s) => s.id !== id);
	}

	/** The server signs out every other session as part of this. */
	async changePassword(currentPassword: string, newPassword: string) {
		await this.#api.users.changePassword({
			current_password: currentPassword,
			new_password: newPassword
		});
		this.sessions = this.sessions.filter((s) => s.current);
	}

	async revokeOtherSessions() {
		await this.#api.users.sessions.revokeOthers();
		this.sessions = this.sessions.filter((s) => s.current);
	}
}

export const currentUser = new CurrentUserStore();
