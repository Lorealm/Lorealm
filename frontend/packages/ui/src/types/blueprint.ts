import type { Snowflake } from './core.js';
import type { RealmResourceKind } from './realm.js';

/** A WTN document with unbound `#inputs`. */
export interface Blueprint {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	source: string;
	created_at: string;
	updated_at: string;
}

export interface BlueprintSummary {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	updated_at: string;
}

export interface CreateBlueprintRequest {
	name: string;
	source?: string;
}

export interface UpdateBlueprintRequest {
	name?: string | null;
	description?: string | null;
	source?: string | null;
}

export type InputSpec = {
	name: string;
	/** The declaration as written, e.g. `[Ref<Character>]`. */
	declared: string;
	required: boolean;
} & InputSpecType;

export type InputSpecType =
	| { type: 'string' }
	| { type: 'number' }
	| { type: 'bool' }
	| { type: 'enum'; variants: string[] }
	| { type: 'list'; of: InputSpecType }
	| { type: 'reference'; kind: RealmResourceKind | null };

/** Values for the declared inputs. */
export interface ExpandBlueprintRequest {
	inputs: Record<string, unknown>;
}
