export type ValueType = 'string' | 'number' | 'bool' | 'array' | 'none' | 'any';

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
