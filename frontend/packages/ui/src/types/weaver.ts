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

export interface MemberDef {
	name: string;
	returns: ValueType;
	description: string;
}

/** A `scope:` an editor can complete against. */
export interface NamespaceDef {
	scope: string;
	description: string;
	members: MemberDef[];
	profiles: WeaverProfile[];
}

export interface WeaverVocabulary {
	commands: CommandSignature[];
	processors: ProcessorSignature[];
	namespaces: NamespaceDef[];
}

/**
 * Which compiler weaver source belongs to. Whole editors hold a
 * `template`; condition fields hold a bare `expression`.
 */
export type WeaverDialect = 'template' | 'expression' | 'document' | 'module';

export type WeaverProfile =
	| 'chat_turn'
	| 'command'
	| 'realm_script'
	| 'realm_condition'
	| 'realm_director'
	| 'widget_render'
	| 'realm_template'
	| 'template_text';

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
