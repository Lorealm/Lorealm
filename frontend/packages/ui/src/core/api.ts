import type { LoginRequest, Session } from '#lib/types/auth.js';

export type KnownCapability = 'placeholder' | 'placeholder2';

export interface PlatformAPI {
	auth: AuthApi;
	http: HttpTransport;
	store: LocalStore;
	files: FileTransfer;
	capabilities: Set<KnownCapability | (string & {})>;
	window?: WindowControls;
	server?: ServerControls;
	/** Absent on platforms that update some other way, like the web build. */
	updates?: UpdateControls;
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

export interface FileTransfer {
	/** Ask for a file and read it. Null when the user picked nothing. */
	open(options: FileDialog): Promise<string | null>;

	/**
	 * Ask for a file and read it as a `data:` URI, bytes and all.
	 */
	openBinary(options: FileDialog): Promise<string | null>;

	/**
	 * Ask where to put `contents` and write it: text as-is, a `Blob` byte for
	 * byte. False when the user cancelled.
	 */
	save(contents: string | Blob, options: FileDialog): Promise<boolean>;
}

export interface FileDialog {
	/** What the dialog calls this kind of file, e.g. "Lorealm style". */
	label: string;

	/** Accepted extensions, without the dot. */
	extensions: string[];

	/** The name to offer when saving. */
	suggestedName?: string;
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

/** Self-updating for installed clients. */
export interface UpdateControls {
	/** The newer release on offer, or null when this build is current. */
	check(): Promise<AvailableUpdate | null>;
}

export interface AvailableUpdate {
	version: string;
	notes?: string;
	/** Downloads and installs the update, then restarts the app. */
	install(): Promise<void>;
}
