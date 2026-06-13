import type { LoginRequest, Session } from '$lib/types/auth.js';

export type KnownCapability = 'placeholder' | 'placeholder2';

export interface PlatformAPI {
	auth: AuthApi;
	http: HttpTransport;
	capabilities: Set<KnownCapability | (string & {})>;
}

export interface AuthApi {
	/** Attempt to restore a session from stored credentials. Returns null if nothing is stored. */
	hydrate(): Promise<Session | null>;
	login(credentials: LoginRequest): Promise<Session>;
	logout(): Promise<void>;
}

export interface HttpTransport {
	/* JSON */
	get<T>(path: string): Promise<T>;
	post<T>(path: string, body?: unknown): Promise<T>;
	put<T>(path: string, body?: unknown): Promise<T>;
	patch<T>(path: string, body?: unknown): Promise<T>;
	delete<T>(path: string): Promise<T>;

	/* Bytes */
	getBlob(path: string): Promise<Blob>;
	putBlob<T>(path: string, blob: Blob): Promise<T>;

	/* SSE stream */
	stream<T>(path: string, body?: unknown): AsyncIterable<T>;
}
