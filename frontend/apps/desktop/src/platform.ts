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

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string | undefined,
		message: string,
		readonly body: unknown
	) {
		super(message);
		this.name = "ApiError";
	}
}

function check<T>(res: HttpResponse): T {
	if (res.status >= 400) {
		const body = res.body;
		let code: string | undefined;
		let message: string;
		if (body && typeof body === "object") {
			code = (body as { error_code?: string }).error_code;
			message = (body as { message?: string }).message ?? `HTTP ${res.status}`;
		} else if (typeof body === "string" && body.trim() !== "") {
			message = body; // plain-text framework error (e.g. serde deserialize message)
		} else {
			message = `HTTP ${res.status}`;
		}
		throw new ApiError(res.status, code, message, body);
	}
	return res.body as T;
}

interface HttpResponse {
	status: number;
	body: unknown;
}

const http: HttpTransport = {
	async get<T>(path: string): Promise<T> {
		return check<T>(await invoke<HttpResponse>("http_get", { path }));
	},
	async post<T>(path: string, body?: unknown): Promise<T> {
		return check<T>(await invoke<HttpResponse>("http_post", { path, body: body ?? null }));
	},
	async put<T>(path: string, body?: unknown): Promise<T> {
		return check<T>(await invoke<HttpResponse>("http_put", { path, body: body ?? null }));
	},
	async delete<T>(path: string): Promise<T> {
		return check<T>(await invoke<HttpResponse>("http_delete", { path }));
	},
};

export const tauriPlatform: PlatformAPI = {
  auth: tauriAuth,
  http,
  capabilities: new SvelteSet([])
}
