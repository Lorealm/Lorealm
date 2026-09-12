import { Channel, invoke } from "@tauri-apps/api/core";
import type {
	AuthApi,
	FileDialog,
	FileTransfer,
	HttpTransport,
	LocalStore,
	PlatformAPI,
	ServerControls,
	Session,
	User,
} from "@lorealm/ui";
import { SvelteSet } from "svelte/reactivity";
import { platform } from "@tauri-apps/plugin-os";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isMac = platform() === "macos";
const w = getCurrentWindow();

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
		readonly body: unknown,
		readonly details?: { resource?: string; id?: string }
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
		let details: { resource?: string; id?: string } | undefined;
		if (body && typeof body === "object") {
			code = (body as { error_code?: string }).error_code;
			message = (body as { message?: string }).message ?? `HTTP ${res.status}`;
			details = (body as { details?: { resource?: string; id?: string } }).details;
		} else if (typeof body === "string" && body.trim() !== "") {
			message = body; // plain-text framework error (e.g. serde deserialize message)
		} else {
			message = `HTTP ${res.status}`;
		}
		throw new ApiError(res.status, code, message, body, details);
	}
	return res.body as T;
}

interface HttpResponse {
	status: number;
	body: unknown;
}

function sniffMime(b: Uint8Array): string {
	if (b.length >= 3 && b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "image/jpeg";
	if (b.length >= 4 && b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return "image/png";
	if (b.length >= 4 && b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46) return "image/gif";
	if (b.length >= 12 && b[8] === 0x57 && b[9] === 0x45 && b[10] === 0x42 && b[11] === 0x50) return "image/webp";
	return "application/octet-stream";
}

type StreamEvent<T> =
	| { kind: "message"; data: T }
	| { kind: "error"; message: string; body?: unknown }
	| { kind: "done" };

	async function* stream<T>(path: string, body?: unknown): AsyncGenerator<T> {
		const channel = new Channel<StreamEvent<T>>();

		const queue: StreamEvent<T>[] = [];
		let wake: (() => void) | null = null;
		let failure: unknown = null;

		channel.onmessage = (msg) => {
			queue.push(msg);
			wake?.();
			wake = null;
		};

		invoke<void>("http_stream", { path, body: body ?? null, onEvent: channel })
			.catch((e) => {
				const err = (e as { HttpStatus?: { status: number; body: unknown } }).HttpStatus;
				if (err) {
					try {
						check<never>({ status: err.status, body: err.body });
					} catch (apiErr) {
						failure = apiErr;
					}
				} else {
					failure = e;
				}
			})
			.finally(() => {
				wake?.();
				wake = null;
			});

		while (true) {
			if (queue.length > 0) {
				const msg = queue.shift()!;
				if (msg.kind === "message") yield msg.data;
				else if (msg.kind === "error") {
					if (msg.body) check<never>({ status: 500, body: msg.body });
					throw new Error(msg.message);
				}
				else return;
			} else if (failure) {
				throw failure; // pre-flight HTTP error, queued before any message
			} else {
				await new Promise<void>((r) => (wake = r));
			}
		}
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
    async patch<T>(path: string, body?: unknown): Promise<T> {
  		return check<T>(await invoke<HttpResponse>("http_patch", { path, body: body ?? null }));
   	},
  	async delete<T>(path: string): Promise<T> {
  		return check<T>(await invoke<HttpResponse>("http_delete", { path }));
    },
    async getBlob(path: string): Promise<Blob> {
     	try {
      		const buf = await invoke<ArrayBuffer>("http_get_blob", { path });
      		const bytes = new Uint8Array(buf);
      		return new Blob([bytes], { type: sniffMime(bytes) });
     	} catch (e) {
      		const err = (e as { HttpStatus?: { status: number; body: unknown } }).HttpStatus;
      		if (err) return check<never>({ status: err.status, body: err.body }); // always throws
      		throw e;
        }
    },
    async putBlob<T>(path: string, blob: Blob): Promise<T> {
     	const buf = await blob.arrayBuffer();
     	const res = await invoke<HttpResponse>("http_put_blob", new Uint8Array(buf), {
      		headers: {
     			"x-path": path,
     			"x-content-type": blob.type || "application/octet-stream",
      		},
     	});
     	return check<T>(res);
    },
    stream
};

const store: LocalStore = {
	get: <T,>(key: string) => invoke<T | null>("store_get", { key }),
	set: (key, value) => invoke("store_set", { key, value }),
	delete: (key) => invoke("store_delete", { key }),
};

const files: FileTransfer = {
	open: (options: FileDialog) =>
		invoke<string | null>("pick_file", {
			label: options.label,
			extensions: options.extensions,
		}),
	save: (contents: string, options: FileDialog) =>
		invoke<boolean>("save_file", {
			contents,
			label: options.label,
			extensions: options.extensions,
			suggestedName: options.suggestedName ?? null,
		}),
};

const server: ServerControls = {
	url: () => invoke<string>("get_server_url"),
	setUrl: (url) => invoke("set_server_url", { url }),
};

export const tauriPlatform: PlatformAPI = {
	auth: tauriAuth,
	http,
	store,
	files,
	server,
	window: isMac
		? undefined
		: {
				minimize: () => w.minimize(),
				toggleMaximize: () => w.toggleMaximize(),
				close: () => w.close(),
			},
	capabilities: new SvelteSet([isMac ? "titlebar-overlay" : "window-controls"]),
};
