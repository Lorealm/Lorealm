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
	/** Can't reach the provider without an API key. False where a local server can stand in. */
	requiresKey: boolean;
	common: CommonParamsSchema;
}

export interface ProviderModel {
	id: string;
	name?: string | null;
	description?: string | null;
	context_length?: number | null;
	pricing?: { prompt?: number | null; completion?: number | null } | null;
}

export const TOKEN_BUDGET: NumberSpec = {
	kind: 'number',
	min: 256,
	max: 1_024_000,
	step: 256,
	default: 8192
};

export const OPENROUTER_REASONING_BUDGET: NumberSpec = {
	kind: 'number',
	min: 256,
	max: 32768,
	step: 256,
	default: 2048
};

export const ANTHROPIC_THINKING_BUDGET: NumberSpec = {
	kind: 'number',
	min: 1024,
	max: 64000,
	step: 256,
	default: 4096
};

export const VERTEX_THINKING_BUDGET: NumberSpec = {
	kind: 'number',
	min: 0,
	max: 32768,
	step: 256,
	default: 8192
};

export const PROVIDER_SCHEMAS: ProviderSchema[] = [
	{
		provider: 'openrouter',
		label: 'OpenRouter',
		requiresKey: true,
		common: {
			temperature: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 },
			top_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 1 },
			max_tokens: { kind: 'number', min: 1, max: 32768, step: 1, default: 4096 },
			stop: { kind: 'string_list' },
			// The params below assume the extended CommonParams (top_k, min_p,
			// penalties).
			top_k: { kind: 'number', min: 0, max: 200, step: 1, default: 0 },
			min_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 0 },
			frequency_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			presence_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			repetition_penalty: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 }
		}
	},
	{
		provider: 'openai',
		label: 'OpenAI-compatible',
		requiresKey: false,
		common: {
			temperature: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 },
			top_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 1 },
			max_tokens: { kind: 'number', min: 1, max: 32768, step: 1, default: 4096 },
			stop: { kind: 'string_list' },
			frequency_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			presence_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			// Not in OpenAI's own schema, but required for compatibility.
			// The server only sends these when they are set.
			top_k: { kind: 'number', min: 0, max: 200, step: 1, default: 0 },
			min_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 0 },
			repetition_penalty: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 }
		}
	},
	{
		provider: 'anthropic',
		label: 'Anthropic',
		requiresKey: true,
		common: {
			temperature: { kind: 'number', min: 0, max: 1, step: 0.01, default: 1 },
			top_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 1 },
			top_k: { kind: 'number', min: 1, max: 500, step: 1, default: 40 },
			// The server's default when unset; the API requires a value.
			max_tokens: { kind: 'number', min: 1, max: 128000, step: 1, default: 16000 },
			stop: { kind: 'string_list' }
		}
	},
	{
		provider: 'vertex',
		label: 'Google Vertex AI',
		requiresKey: true,
		common: {
			temperature: { kind: 'number', min: 0, max: 2, step: 0.01, default: 1 },
			top_p: { kind: 'number', min: 0, max: 1, step: 0.01, default: 0.95 },
			top_k: { kind: 'number', min: 1, max: 200, step: 1, default: 64 },
			max_tokens: { kind: 'number', min: 1, max: 65536, step: 1, default: 8192 },
			stop: { kind: 'string_list' },
			frequency_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 },
			presence_penalty: { kind: 'number', min: -2, max: 2, step: 0.01, default: 0 }
		}
	}
];

export function getProviderSchema(provider: string): ProviderSchema | undefined {
	return PROVIDER_SCHEMAS.find((s) => s.provider === provider);
}
