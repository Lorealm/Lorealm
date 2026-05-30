import { invoke } from "@tauri-apps/api/core";
import type { AuthApi, HttpTransport, PlatformAPI, Session, User } from "@eldoria-hub/ui";
import { SvelteSet } from "svelte/reactivity";

const tauriAuth: AuthApi = {
	async hydrate() {
		console.log("[HYDRATE] Hydrating authentication state");
		// Read stored token, then validate it against your server
		const token = await invoke<string | null>("get_stored_token");
		if (!token) throw new Error("No token found");

		try {
			const user = await invoke<User>("validate_token");
			return { token, user };
		} catch (e) {
			console.log("[HYDRATE] Error encountered: " + JSON.stringify(e));
			await invoke("clear_stored_token");
			return null;
		}
	},

  async login(credentials) {
    console.log("[LOGIN] Attempting login");
		const session = await invoke<Session>("login", {
			username: credentials.username,
			password: credentials.password,
		});
		return session;
	},

	async logout() {
		await invoke("clear_stored_token");
	},
};

interface HttpResponse {
	status: number;
	body: unknown;
}

const http: HttpTransport = {
	async get<T>(path: string): Promise<T> {
		const res = await invoke<HttpResponse>("http_get", { path });
		return res.body as T;
	},
	async post<T>(path: string, body?: unknown): Promise<T> {
		const res = await invoke<HttpResponse>("http_post", {
			path,
			body: body ?? null,
		});
		return res.body as T;
	},
	async put<T>(path: string, body?: unknown): Promise<T> {
		const res = await invoke<HttpResponse>("http_put", {
			path,
			body: body ?? null,
		});
		return res.body as T;
	},
	async delete<T>(path: string): Promise<T> {
		const res = await invoke<HttpResponse>("http_delete", { path });
		return res.body as T;
	},
};

export const tauriPlatform: PlatformAPI = {
  auth: tauriAuth,
  http,
  capabilities: new SvelteSet([])
}
