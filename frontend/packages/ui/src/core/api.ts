export type KnownCapability = 'placeholder' | 'placeholder2';

export interface PlatformAPI {
	http: HttpTransport;
	capabilities: Set<KnownCapability | (string & {})>;
}

export interface HttpTransport {
	get<T>(path: string): Promise<T>;
	post<T>(path: string, body?: unknown): Promise<T>;
	put<T>(path: string, body?: unknown): Promise<T>;
	delete<T>(path: string): Promise<T>;
}
