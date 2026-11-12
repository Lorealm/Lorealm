import type { Snowflake } from './core.js';
import type { AgentAccess, Director, RealmResourceKind } from './realm.js';

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
	/** The `//` lines above the declaration: what the author says it is for. */
	doc: string | null;
} & InputSpecType;

/** What someone building a realm from a blueprint is shown. */
export interface BuildForm {
	/** The author's markdown, shown before the fields. */
	introduction: string | null;
	inputs: InputSpec[];
}

export type InputSpecType =
	| { type: 'string' }
	| { type: 'number' }
	| { type: 'bool' }
	| { type: 'enum'; variants: string[] }
	| { type: 'range'; min: number; max: number }
	| { type: 'span'; min: number; max: number }
	| { type: 'list'; of: InputSpecType }
	| { type: 'object'; fields: Array<{ name: string } & InputSpecType> }
	/** `ref_name` is the kind as written in `Ref<...>`; `kind` is set when it's a built-in one. */
	| { type: 'reference'; kind: RealmResourceKind | null; ref_name: string };

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
	introduction: string;
	tags: string[];
	director: Director | null;
	scenarios: OutlineScenario[];
	config: Record<string, string>;
	inputs: InputSpec[];
	cast: OutlineEntry[];
	modules: OutlineBinding[];
}

/** Why a document cannot be shown as a block model. */
export interface OutlineRefusal {
	message: string;
	from: number | null;
	to: number | null;
}

export interface OutlineScenario {
	slug: string;
	name: string;
	description: string;
	opening: string;
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
	widgets: OutlineBinding[];
	tools: OutlineTool[];
	grants: OutlineGrant[];
	properties: OutlineProperty[];
	speech_channel: string;
	script: string;
	run_condition: string;
}

export interface OutlineTool {
	tool: OutlineBinding;
	event_channel: string;
}

/** The grant's pattern text; the editor splits it into controls. */
export interface OutlineGrant {
	channel: string;
	access: AgentAccess;
}

/** Text the editor can re-read; there is no type on the wire. */
export interface OutlineProperty {
	key: string;
	value: string;
}

export type OutlineBinding =
	| { source: 'resource'; kind: RealmResourceKind; id: Snowflake }
	| { source: 'loop'; varName: string; path: string[] }
	| { source: 'input'; name: string; path: string[] };

export interface BundlePreview {
	name: string;
	description: string;
	resources: BundleResource[];
}

export interface BundleResource {
	kind: RealmResourceKind;
	name: string;
	status: 'new' | 'existing' | 'blocked';
	problem?: string;
	includes: { kind: RealmResourceKind; name: string }[];
	has_image: boolean;
}

export interface ImportedBundle {
	blueprint: Blueprint;
	resources: { kind: RealmResourceKind; name: string; id: Snowflake; reused: boolean }[];
	/** Tools saved under a new name: `[original, new]`. */
	renamed: [string, string][];
	skipped: string[];
}
