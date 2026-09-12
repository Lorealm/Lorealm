import type {
	AuthApi,
	FileDialog,
	FileTransfer,
	HttpTransport,
	LocalStore,
	PlatformAPI,
	Session,
	User,
} from "@lorealm/ui";
import { SvelteSet } from "svelte/reactivity";

const BASE = import.meta.env.VITE_API_BASE_URL ?? "";

const CSRF_HEADER = "X-CSRF-Token";
const CSRF_STORAGE_KEY = "csrf_token";

let csrfToken: string | null =
	typeof localStorage !== "undefined" ? localStorage.getItem(CSRF_STORAGE_KEY) : null;

function setCsrf(token: string | null) {
	csrfToken = token;
	if (typeof localStorage === "undefined") return;
	if (token) localStorage.setItem(CSRF_STORAGE_KEY, token);
	else localStorage.removeItem(CSRF_STORAGE_KEY);
}

function csrfHeaders(): Record<string, string> {
	return csrfToken ? { [CSRF_HEADER]: csrfToken } : {};
}

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

interface HttpResponse {
	status: number;
	body: unknown;
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
			message = body;
		} else {
			message = `HTTP ${res.status}`;
		}
		throw new ApiError(res.status, code, message, body, details);
	}
	return res.body as T;
}

async function readBody(res: Response): Promise<unknown> {
	const text = await res.text();
	if (text === "") return null;
	try {
		return JSON.parse(text);
	} catch {
		return text;
	}
}

async function handle<T>(res: Response): Promise<T> {
	return check<T>({ status: res.status, body: await readBody(res) });
}

function tryParseJson(data: string): unknown {
	try {
		return JSON.parse(data);
	} catch {
		return data;
	}
}

function parseSseEvent(raw: string): { event: string; data: string } | null {
	let event = "message";
	const dataLines: string[] = [];
	for (let line of raw.split("\n")) {
		if (line.endsWith("\r")) line = line.slice(0, -1);
		if (line === "" || line.startsWith(":")) continue; // blank / comment
		const idx = line.indexOf(":");
		const field = idx === -1 ? line : line.slice(0, idx);
		let value = idx === -1 ? "" : line.slice(idx + 1);
		if (value.startsWith(" ")) value = value.slice(1);
		if (field === "event") event = value;
		else if (field === "data") dataLines.push(value);
	}
	if (dataLines.length === 0) return null;
	return { event, data: dataLines.join("\n") };
}

async function* stream<T>(path: string, body?: unknown): AsyncGenerator<T> {
	const res = await fetch(`${BASE}${path}`, {
		method: "POST",
		credentials: "include",
		headers: {
			"Content-Type": "application/json",
			Accept: "text/event-stream",
			...csrfHeaders(),
		},
		body: JSON.stringify(body ?? null),
	});

	if (res.status >= 400) {
		await handle<never>(res); // always throws
		return;
	}
	if (!res.body) return;

	const reader = res.body.getReader();
	const decoder = new TextDecoder();
	let buffer = "";

	try {
		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			buffer += decoder.decode(value, { stream: true });
			buffer = buffer.replace(/\r\n/g, "\n");

			let sep: number;
			while ((sep = buffer.indexOf("\n\n")) !== -1) {
				const rawEvent = buffer.slice(0, sep);
				buffer = buffer.slice(sep + 2);

				const parsed = parseSseEvent(rawEvent);
				if (!parsed) continue;

				const data = tryParseJson(parsed.data);
				if (parsed.event === "error") {
					throw check<never>({ status: 500, body: data });
				}
				yield data as T;
			}
		}
	} finally {
		reader.releaseLock();
	}
	// Byte stream ended without an explicit terminator -> done.
}

const http: HttpTransport = {
	async get<T>(path: string): Promise<T> {
		return handle<T>(await fetch(`${BASE}${path}`, { credentials: "include" }));
	},
	async post<T>(path: string, body?: unknown): Promise<T> {
		return handle<T>(
			await fetch(`${BASE}${path}`, {
				method: "POST",
				credentials: "include",
				headers: { "Content-Type": "application/json", ...csrfHeaders() },
				body: JSON.stringify(body ?? null),
			})
		);
	},
	async put<T>(path: string, body?: unknown): Promise<T> {
		return handle<T>(
			await fetch(`${BASE}${path}`, {
				method: "PUT",
				credentials: "include",
				headers: { "Content-Type": "application/json", ...csrfHeaders() },
				body: JSON.stringify(body ?? null),
			})
		);
	},
	async patch<T>(path: string, body?: unknown): Promise<T> {
		return handle<T>(
			await fetch(`${BASE}${path}`, {
				method: "PATCH",
				credentials: "include",
				headers: { "Content-Type": "application/json", ...csrfHeaders() },
				body: JSON.stringify(body ?? null),
			})
		);
	},
	async delete<T>(path: string): Promise<T> {
		return handle<T>(
			await fetch(`${BASE}${path}`, {
				method: "DELETE",
				credentials: "include",
				headers: { ...csrfHeaders() },
			})
		);
	},
	async getBlob(path: string): Promise<Blob> {
		const res = await fetch(`${BASE}${path}`, { credentials: "include" });
		if (res.status >= 400) await handle<never>(res); // always throws
		return res.blob(); // browser sets Blob.type from Content-Type
	},
	async putBlob<T>(path: string, blob: Blob): Promise<T> {
		return handle<T>(
			await fetch(`${BASE}${path}`, {
				method: "PUT",
				credentials: "include",
				headers: {
					"Content-Type": blob.type || "application/octet-stream",
					...csrfHeaders(),
				},
				body: blob,
			})
		);
	},
	stream,
};

const webAuth: AuthApi = {
	async hydrate() {
		try {
			const user = await http.get<User>("/api/v1/users/me");
			return { token: "", user };
		} catch {
			return null;
		}
	},

	async login(credentials) {
		const session = await http.post<Session>("/api/v1/users/login", {
			username: credentials.username,
			password: credentials.password,
		});
		const csrf = (session as unknown as { csrf_token?: string }).csrf_token;
		if (csrf) setCsrf(csrf);
		return session;
	},

	async logout() {
		try {
			await http.post("/api/v1/users/logout");
		} catch {
			/* ignore */
		}
		setCsrf(null);
	},
};


const STORE_PREFIX = "lorealm:";

const store: LocalStore = {
	async get<T>(key: string): Promise<T | null> {
		if (typeof localStorage === "undefined") return null;
		const raw = localStorage.getItem(STORE_PREFIX + key);
		if (raw === null) return null;
		try {
			return JSON.parse(raw) as T;
		} catch {
			return null; // Corrupt entry: fall back to the caller's defaults.
		}
	},
	async set(key, value) {
		if (typeof localStorage === "undefined") return;
		localStorage.setItem(STORE_PREFIX + key, JSON.stringify(value));
	},
	async delete(key) {
		if (typeof localStorage === "undefined") return;
		localStorage.removeItem(STORE_PREFIX + key);
	},
};

const files: FileTransfer = {
	open(options: FileDialog): Promise<string | null> {
		return new Promise((resolve, reject) => {
			const input = document.createElement("input");
			input.type = "file";
			input.accept = options.extensions.map((e) => `.${e}`).join(",");

			input.oncancel = () => resolve(null);
			input.onchange = () => {
				const file = input.files?.[0];
				if (!file) return resolve(null);
				file.text().then(resolve, reject);
			};

			input.click();
		});
	},

	openBinary(options: FileDialog): Promise<string | null> {
		return new Promise((resolve, reject) => {
			const input = document.createElement("input");
			input.type = "file";
			input.accept = options.extensions.map((e) => `.${e}`).join(",");

			input.oncancel = () => resolve(null);
			input.onchange = () => {
				const file = input.files?.[0];
				if (!file) return resolve(null);

				const reader = new FileReader();
				reader.onload = () => resolve(reader.result as string);
				reader.onerror = () => reject(reader.error);
				reader.readAsDataURL(file);
			};

			input.click();
		});
	},

	async save(contents: string, options: FileDialog): Promise<boolean> {
		const url = URL.createObjectURL(new Blob([contents], { type: "application/json" }));
		const link = document.createElement("a");
		link.href = url;
		link.download = options.suggestedName ?? `download.${options.extensions[0] ?? "txt"}`;
		link.click();

    setTimeout(() => URL.revokeObjectURL(url), 60_000);
		
		return true;
	},
};

export const webPlatform: PlatformAPI = {
	auth: webAuth,
	http,
	store,
	files,
	capabilities: new SvelteSet([]),
};
