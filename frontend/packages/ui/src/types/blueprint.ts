import type { Snowflake } from './core.js';
import type { Director, RealmResourceKind } from './realm.js';

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

/** A document read back into the editor's block model. */
export type OutlineResult =
	| { status: 'ok'; outline: Outline }
	| { status: 'refused'; reason: OutlineRefusal };

export interface Outline {
	name: string;
	description: string;
	tags: string[];
	director: Director | null;
	inputs: InputSpec[];
	cast: OutlineEntry[];
}

/** Why a document cannot be shown as a block model. */
export interface OutlineRefusal {
	message: string;
	from: number | null;
	to: number | null;
}

export type OutlineEntry =
	| { kind: 'agent'; agent: OutlineAgent }
	| { kind: 'foreach'; var_name: string; over: string | null; body: OutlineAgent[] };

export interface OutlineAgent {
	handle: string;
	name: string;
	kind: string;
	character: OutlineBinding | null;
	template: OutlineBinding | null;
	persona: OutlineBinding | null;
	profile: OutlineBinding | null;
	lorebooks: OutlineBinding[];
	script: string;
	run_condition: string;
}

export type OutlineBinding =
	| { source: 'resource'; kind: RealmResourceKind; id: Snowflake }
	| { source: 'loop'; varName: string }
	| { source: 'input'; name: string };
