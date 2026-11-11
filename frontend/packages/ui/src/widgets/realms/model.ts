import type { Snowflake } from '$lib/types/core.js';
import type { AgentAccess, AgentKind, RealmResourceKind } from '$lib/types/realm.js';

/** The fields a `Ref<..>` projects.
 *
 * VOLATILE: Mirrors `projection()` in `project.rs`. Could change without notice. */
export const PROJECTED_FIELDS = ['id', 'name', 'description', 'slug'] as const;

/**
 * What sits in a resource-shaped field. Either the author picked something
 * concrete, or it reads off a loop variable, or off a singular entity input.
 */
export type Binding =
	| { source: 'resource'; kind: RealmResourceKind; id: Snowflake; name: string }
	| { source: 'loop'; varName: string; path: string[] }
	| { source: 'input'; name: string; path: string[] };

/** An `#inputs` declaration. */
export interface InputDecl {
	name: string;
	ty: InputType;
	defaultValue: string | null;
}

export function isRequired(decl: InputDecl): boolean {
	return decl.defaultValue === null;
}

export type InputType =
	| { type: 'string' }
	| { type: 'number' }
	| { type: 'bool' }
	| { type: 'enum'; variants: string[] }
	/** A number inside an inclusive space: one slider handle. */
	| { type: 'range'; min: number; max: number }
	/** A `{from, to}` pair inside an inclusive space: two handles. */
	| { type: 'span'; min: number; max: number }
	| { type: 'list'; of: InputType }
	| { type: 'object'; fields: InputField[] }
	/** `kind: null` is a `Ref<..>` naming something the server doesn't know. */
	| { type: 'reference'; kind: RealmResourceKind | null };

export interface InputField {
	name: string;
	ty: InputType;
}

export interface Grant {
	namespace: string;
	/** `*`, `name`, or `name.*`. */
	scope: string;
	access: AgentAccess;
}

/** A bound tool, and the channel its `acted` event goes out on. */
export interface ToolBinding {
	tool: Binding;
	/** Empty means only the acting agent observes it. */
	eventChannel: string;
}

export interface Property {
	key: string;
	value: string;
}

export interface ScenarioBlock {
	/** Local identity for the editing list. Never written to the document. */
	readonly key: string;
	slug: string;
	name: string;
	description: string;
	opening: string;
}

export function emptyScenario(): ScenarioBlock {
	return { key: freshKey('scenario-'), slug: '', name: '', description: '', opening: '' };
}

export interface AgentBlock {
	/** Local identity for keyed lists and selection. */
	readonly key: string;
	handle: string;
	name: string;
	kind: AgentKind;
	character: Binding | null;
	template: Binding | null;
	persona: Binding | null;
	profile: Binding | null;
	lorebooks: Binding[];
	widgets: Binding[];
	tools: ToolBinding[];
	grants: Grant[];
	properties: Property[];
	speechChannel: string;
	script: string;
	run_condition: string;
}

/**
 * One entry in the cast. A `foreach` emits its body once per element of the
 * input it walks, which is the whole of what makes a blueprint a blueprint.
 */
export type CastEntry =
	| { readonly key: string; kind: 'agent'; agent: AgentBlock }
	| {
			readonly key: string;
			kind: 'foreach';
			varName: string;
			over: string | null;
			body: AgentBlock[];
	  };

/** What a pane or the inspector is currently pointed at. */
export type Selection =
	| { kind: 'agent'; key: string }
	| { kind: 'foreach'; key: string }
	| { kind: 'input'; name: string }
	/** A resource pinged from a field */
	| { kind: 'resource'; resourceKind: RealmResourceKind; id: Snowflake; name: string }
	| null;

export function selectionId(selection: NonNullable<Selection>): string {
	switch (selection.kind) {
		case 'input':
			return `input:${selection.name}`;
		case 'resource':
			return `resource:${selection.resourceKind}:${selection.id}`;
		default:
			return `${selection.kind}:${selection.key}`;
	}
}

let counter = 0;
export function freshKey(prefix: string): string {
	return `${prefix}${++counter}`;
}

export function emptyAgent(handle: string, kind: AgentKind = 'llm'): AgentBlock {
	return {
		key: freshKey('agent-'),
		handle,
		name: '',
		kind,
		character: null,
		template: null,
		persona: null,
		profile: null,
		lorebooks: [],
		widgets: [],
		tools: [],
		grants: [],
		properties: [],
		speechChannel: '',
		script: '',
		run_condition: ''
	};
}

/** The declaration as an author writes it, e.g. `[Ref<Character>]`. */
export function declaredText(ty: InputType): string {
	switch (ty.type) {
		case 'string':
		case 'number':
		case 'bool':
			return ty.type;
		case 'enum':
			return `enum(${ty.variants.map((v) => `"${v}"`).join(', ')})`;
		case 'range':
			return `range(${ty.min}, ${ty.max})`;
		case 'span':
			return `span(${ty.min}, ${ty.max})`;
		case 'list':
			return `[${declaredText(ty.of)}]`;
		case 'object':
			return `{${ty.fields.map((f) => `${f.name}: ${declaredText(f.ty)}`).join(', ')}}`;
		case 'reference':
			return `Ref<${ty.kind ? REF_NAME[ty.kind] : 'Unknown'}>`;
	}
}

export function browseText(ty: InputType): string {
	if (ty.type === 'object') return 'Custom Object';
	if (ty.type === 'list') return `[${browseText(ty.of)}]`;
	return declaredText(ty);
}

/**
 * Every reference inside a type, and the field path that reaches it.
 */
export function referencePaths(
	ty: InputType,
	prefix: string[] = []
): Array<{
	path: string[];
	kind: RealmResourceKind | null;
}> {
	switch (ty.type) {
		case 'reference':
			return [{ path: prefix, kind: ty.kind }];
		case 'object':
			return ty.fields.flatMap((f) => referencePaths(f.ty, [...prefix, f.name]));
		default:
			return [];
	}
}

/** The name a `Ref<..>` writes for each kind. Mirrors `RealmResourceKind::ref_name`. */
export const REF_NAME: Record<RealmResourceKind, string> = {
	character: 'Character',
	lorebook: 'Lorebook',
	template: 'Template',
	tool: 'Tool',
	persona: 'Persona',
	connection_profile: 'ConnectionProfile',
	widget: 'Widget',
	module: 'Module'
};

/** `REF_NAME` is the notation's spelling; this is what a person calls it. */
export const TYPE_NAME: Record<RealmResourceKind, string> = {
	character: 'Character',
	lorebook: 'Lorebook',
	template: 'Context Template',
	persona: 'Persona',
	tool: 'Tool',
	connection_profile: 'Connection',
	widget: 'Widget',
	module: 'Module'
};

export const KIND_LABEL: Record<RealmResourceKind, string> = {
	character: 'Characters',
	lorebook: 'Lorebooks',
	template: 'Templates',
	tool: 'Tools',
	persona: 'Personas',
	connection_profile: 'Connections',
	widget: 'Widgets',
	module: 'Modules'
};

export function isList(ty: InputType): boolean {
	return ty.type === 'list';
}

/** The element type with *exactly one* list wrapper peeled off. */
export function baseOf(ty: InputType): InputType {
	return ty.type === 'list' ? ty.of : ty;
}

export function withList(base: InputType, many: boolean): InputType {
	return many ? { type: 'list', of: base } : base;
}

/**
 * One select's worth of vocabulary
 */
export function holdsValue(ty: InputType): string {
	const base = baseOf(ty);
	return base.type === 'reference' ? `ref:${base.kind ?? 'character'}` : base.type;
}

export function blankField(): InputField {
	return { name: 'field', ty: { type: 'string' } };
}

export function holdsToBase(value: string): InputType {
	if (value.startsWith('ref:')) {
		return { type: 'reference', kind: value.slice(4) as RealmResourceKind };
	}
	if (value === 'enum') return { type: 'enum', variants: [] };
	if (value === 'range') return { type: 'range', min: 0, max: 1 };
	if (value === 'span') return { type: 'span', min: 0, max: 10 };
	if (value === 'object') return { type: 'object', fields: [blankField()] };
	return { type: value } as InputType;
}

/**
 * The resource kind a loop over this input yields
 */
export function elementKind(ty: InputType): RealmResourceKind | null {
	if (ty.type !== 'list' || ty.of.type !== 'reference') return null;
	return ty.of.kind;
}

/** What a loop over this input can reach */
export function loopReaches(ty: InputType) {
	return ty.type === 'list' ? referencePaths(ty.of) : [];
}

/** The declaration line as it would be written into `#inputs`. */
export function declarationLine(decl: InputDecl): string {
	const head = `${decl.name}: ${declaredText(decl.ty)}`;
	if (decl.defaultValue === null) return head;
	const base = baseOf(decl.ty);
	const literal =
		base.type === 'number' || base.type === 'bool' ? decl.defaultValue : `"${decl.defaultValue}"`;
	return `${head} = ${literal}`;
}

export function grantChannel(grant: Grant): string {
	const scope = grant.scope.trim();
	return scope ? `${grant.namespace}:${scope}` : grant.namespace;
}

export function parseGrant(channel: string, access: AgentAccess): Grant {
	const at = channel.indexOf(':');
	if (at < 0) return { namespace: channel, scope: '*', access };
	return { namespace: channel.slice(0, at), scope: channel.slice(at + 1) || '*', access };
}

/** A name reduced to a weaver identifier. */
export function slug(name: string): string {
	let out = '';
	for (const ch of name) {
		if (/[A-Za-z0-9]/.test(ch)) out += ch.toLowerCase();
		else if (!out.endsWith('_')) out += '_';
	}
	const trimmed = out.replace(/^_+|_+$/g, '');
	if (trimmed === '') return 'unnamed';
	return /^[0-9]/.test(trimmed) ? `_${trimmed}` : trimmed;
}

/** How a binding reads in a field, as in, the weaver it will be written as. */
export function bindingLabel(binding: Binding): string {
	if (binding.source === 'resource') return binding.name || binding.id;
	const reach = [...binding.path, 'id'].join('.');
	return binding.source === 'loop'
		? `${binding.varName}.${reach}`
		: `input:${binding.name}.${reach}`;
}

/**
 * The resource kind this input can supply, list or not.
 */
export function referencedKind(ty: InputType): RealmResourceKind | null {
	const base = baseOf(ty);
	return base.type === 'reference' ? base.kind : null;
}

/**
 * The scalar leaves of an input and how each is reached
 */
export function leafPaths(
	ty: InputType,
	prefix: string[] = []
): Array<{ path: string[]; declared: string }> {
	const base = baseOf(ty);
	if (base.type !== 'object')
		return prefix.length ? [{ path: prefix, declared: browseText(ty) }] : [];
	return base.fields.flatMap((f) => leafPaths(f.ty, [...prefix, f.name]));
}

/** An input whose value fills resource fields without a loop around it. */
export function isSingular(ty: InputType): boolean {
	return !isList(ty) && referencePaths(baseOf(ty)).length > 0;
}
