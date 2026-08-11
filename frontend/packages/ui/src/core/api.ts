import type { LoginRequest, Session } from '$lib/types/auth.js';

export type KnownCapability = 'placeholder' | 'placeholder2';

export interface PlatformAPI {
	auth: AuthApi;
	http: HttpTransport;
	store: LocalStore;
	capabilities: Set<KnownCapability | (string & {})>;
	window?: WindowControls;
	server?: ServerControls;
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

export interface WindowControls {
	minimize(): Promise<void>;
	toggleMaximize(): Promise<void>;
	close(): Promise<void>;
}

/**
 * Key-value storage that outlives the session, private to this client.
 */
export interface LocalStore {
	/** The stored value, or null when the key was never written. */
	get<T>(key: string): Promise<T | null>;
	set(key: string, value: unknown): Promise<void>;
	delete(key: string): Promise<void>;
}

/** The address the client talks to. Persisted by the platform, not by {@link LocalStore}. */
export interface ServerControls {
	url(): Promise<string>;
	setUrl(url: string): Promise<void>;
}
