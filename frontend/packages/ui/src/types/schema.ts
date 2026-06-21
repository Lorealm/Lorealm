/**
 * Provider settings schema — frontend-owned by design.
 *
 * Under the option-1 model every control is hand-written in ProfileEditor;
 * this file is the single place that says which controls a provider reveals
 * and what their constraints are. It is version-locked to the controls that
 * consume it, so it lives here rather than on the server.
 *
 * The server owns what is genuinely dynamic instead:
 *   - which providers exist on this instance (future GET /providers)
 *   - model lists, via LlmProvider::list_models()
 *     (GET /api/v1/connections/providers/{provider}/models)
 */

export interface NumberSpec {
	kind: 'number';
	min: number;
	max: number;
	step: number;
	/** Value used by the provider when the param is unset. Shown as placeholder state. */
	default: number;
}

export interface StringListSpec {
	kind: 'string_list';
}

/**
 * Every common param the editor pre-defines a control for. Presence = visible.
 * Mirrors the providers crate's `CommonParams` struct — keep in sync.
 * (`seed` exists server-side in the proposed extension but has no slider
 * semantics; add a plain integer control if/when wanted.)
 */
export interface CommonParamsSchema {
	temperature?: NumberSpec;
	top_p?: NumberSpec;
	top_k?: NumberSpec;
	min_p?: NumberSpec;
	max_tokens?: NumberSpec;
	frequency_penalty?: NumberSpec;
	presence_penalty?: NumberSpec;
	repetition_penalty?: NumberSpec;
	stop?: StringListSpec;
}

export interface ProviderSchema {
	provider: string;
	label: string;
	common: CommonParamsSchema;
}

/** Mirror of the providers crate's `ModelInfo`. Consider moving to $lib/types/connection.ts. */
export interface ProviderModel {
	id: string;
	name?: string | null;
	description?: string | null;
	context_length?: number | null;
	pricing?: { prompt?: number | null; completion?: number | null } | null;
}

// Mirror OPENROUTER_REASONING_BUDGET's exact shape; these values are placeholders.
export const TOKEN_BUDGET: NumberSpec = {
	kind: 'number',
	min: 256,
	max: 1_024_000,
	step: 256,
	default: 8192
};

/** Constraints for OpenRouter's reasoning token budget (provider section control). */
export const OPENROUTER_REASONING_BUDGET: NumberSpec = {
	kind: 'number',
	min: 256,
	max: 32768,
	step: 256,
	default: 2048
};

export const PROVIDER_SCHEMAS: ProviderSchema[] = [
	{
		provider: 'openrouter',
		label: 'OpenRouter',
		common: {
			temperature: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 },
			top_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 1 },
			max_tokens: { kind: 'number', min: 1, max: 32768, step: 1, default: 4096 },
			stop: { kind: 'string_list' },
			// The params below assume the extended CommonParams (top_k, min_p,
			// penalties). If you don't adopt that change, remove them — values
			// would be silently dropped by the server otherwise.
			top_k: { kind: 'number', min: 0, max: 200, step: 1, default: 0 },
			min_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 0 },
			frequency_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			presence_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			repetition_penalty: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 }
		}
	}
];

export function getProviderSchema(provider: string): ProviderSchema | undefined {
	return PROVIDER_SCHEMAS.find((s) => s.provider === provider);
}
