export type ValueType = 'string' | 'number' | 'bool' | 'array' | 'object' | 'none' | 'any';

export interface ParamDef {
	name: string;
	description: string;
	expected_type: ValueType | null;
	required: boolean;
}

export interface PropertyDef {
	key: string;
	description: string;
	expected_type: ValueType | null;
	required: boolean;
}

/** A `$[name(...)]` command. */
export interface CommandSignature {
	name: string;
	description: string;
	returns: ValueType;
	params: ParamDef[];
	/** Whether calling it can change evaluation state. */
	mutates_context: boolean;
}

/** A `@[namespace.name(...)]` processor. */
export interface ProcessorSignature {
	namespace: string;
	name: string;
	description: string;
	returns: ValueType;
	properties: PropertyDef[];
}

export interface WeaverVocabulary {
	commands: CommandSignature[];
	processors: ProcessorSignature[];
}

/**
 * Which compiler weaver source belongs to. Whole editors hold a
 * `template`; condition fields hold a bare `expression`.
 */
export type WeaverDialect = 'template' | 'expression';

/**
 * One problem found in weaver source. `from`/`to` are offsets in UTF-16
 * code units, so they index a JavaScript string directly.
 */
export interface WeaverDiagnostic {
	from: number;
	to: number;
	severity: 'error' | 'warning';
	message: string;
	hint: string | null;
}

export interface WeaverCheckResponse {
	diagnostics: WeaverDiagnostic[];
}
