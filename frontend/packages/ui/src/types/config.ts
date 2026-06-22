export type SlidingPolicy = 'Fixed' | 'Always' | { Window: number };

export type CsrfRotationPolicy = 'Never' | { Periodic: number } | { AfterRequests: number };

export interface HostingConfig {
	ip: string;
	port: number;
}

export type BackendConfig =
	| { type: 'postgres'; connection_string?: string | null; max_connections: number }
	| { type: 'memory'; max_size: number };

export interface CacheConfig {
	redis_url: string;
	ttl_seconds: number;
	key_prefix: string;
}

export interface StorageConfig {
	backends: BackendConfig[];
	data_directory: string;
	require_postgres_fallback: boolean;
	cache?: CacheConfig | null;
}

export interface SessionConfig {
	token_length: number;
	csrf_token_length: number;
	ttl_seconds: number;
	sliding_policy: SlidingPolicy;
	csrf_rotation_policy: CsrfRotationPolicy;
}

export interface MinMax {
	min: number;
	max: number;
}

export interface UserConfig {
	password_length: MinMax;
	username_length: MinMax;
	disable_register: boolean;
	create_default_resources: boolean;
}

export interface CharacterConfig {
	upload_limit_mb: number;
	edit_session_ttl: number;
}

export interface TokenizerConfig {
	max_tokenizer_size_mb: number;
	tokenizer_ttl: number;
	default_tokenizers: string[];
}

export interface TaskConfig {
	cleanup_interval: number;
}

export interface ServerConfig {
	hosting: HostingConfig;
	storage: StorageConfig;
	sessions: SessionConfig;
	users: UserConfig;
	characters: CharacterConfig;
	tokenizers: TokenizerConfig;
	tasks: TaskConfig;
}

export type ConfigKey = keyof ServerConfig;
