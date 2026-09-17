import type { AgentFilter, Director } from '$lib/types/realm.js';
import type {
	AgentBlock,
	Binding,
	CastEntry,
	InputDecl,
	InputType,
	ScenarioBlock
} from './model.js';
import { baseOf, declaredText, grantChannel, isList } from './model.js';

const INDENT = '  ';

function indent(depth: number): string {
	return INDENT.repeat(depth);
}

export function raw(value: string): string {
	let longest = 0;
	for (let i = 0; i < value.length; i++) {
		if (value[i] !== '"') continue;
		let hashes = 0;
		while (value[i + 1 + hashes] === '#') hashes++;
		longest = Math.max(longest, hashes + 1);
	}
	const fence = '#'.repeat(longest);
	return `r${fence}"${value}"${fence}`;
}

/** A quoted string. Expressions are expanded. */
function interpolated(value: string): string {
	return `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
}

/** True where the text carries weaver that should run during expansion. */
function hasInterpolation(value: string): boolean {
	return value.includes('{{') || value.includes('{#');
}

/** A string field: expanded when it reads an input or a loop, raw otherwise. */
function text(value: string): string {
	return hasInterpolation(value) ? interpolated(value) : raw(value);
}

function idField(binding: Binding | null): string {
	if (binding === null) return 'null';
	if (binding.source === 'resource') return raw(binding.id);
	const reach = [...binding.path, 'id'].join('.');
	return binding.source === 'loop'
		? `{{${binding.varName}.${reach}}}`
		: `{{input:${binding.name}.${reach}}}`;
}

function key(name: string): string {
	return /^[A-Za-z_][A-Za-z0-9_]*$/.test(name) ? name : interpolated(name);
}

const NUMBER = /^-?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/;
const QUOTED = /^(["'])([\s\S]*)\1$/;

export function literal(value: string): string {
	const text = value.trim();
	if (text === '') return raw('');
	if (text === 'true' || text === 'false') return text;
	if (NUMBER.test(text)) return text;

	const quoted = QUOTED.exec(text);
	if (quoted) return raw(quoted[2]);

	if (text.startsWith('[') || text.startsWith('{')) return text;

	return raw(value);
}

function agent(block: AgentBlock, depth: number): string {
	const pad = indent(depth + 1);
	const lines: string[] = [];

	lines.push(`${pad}char_version_id: null`);
	lines.push(`${pad}character_id: ${idField(block.kind === 'llm' ? block.character : null)}`);
	lines.push(`${pad}handle: ${text(block.handle)}`);
	lines.push(`${pad}kind: ${raw(block.kind)}`);
	if (block.lorebooks.length > 0) {
		const books = block.lorebooks.map((b) => `${indent(depth + 2)}${idField(b)}`).join('\n');
		lines.push(`${pad}lorebooks: [\n${books}\n${pad}]`);
	}
	lines.push(`${pad}name: ${text(block.name)}`);
	lines.push(`${pad}persona_id: ${idField(block.kind === 'player' ? block.persona : null)}`);
	lines.push(`${pad}profile_id: ${idField(block.kind === 'script' ? null : block.profile)}`);
	if (block.grants.length > 0) {
		const rows = block.grants
			.filter((g) => g.scope.trim())
			.map(
				(g) => `${indent(depth + 2)}{ channel: ${raw(grantChannel(g))}, access: ${raw(g.access)} }`
			);
		if (rows.length > 0) lines.push(`${pad}grants: [\n${rows.join('\n')}\n${pad}]`);
	}
	if (block.properties.length > 0) {
		const rows = block.properties
			.filter((p) => p.key.trim())
			.map((p) => `${indent(depth + 2)}${key(p.key)}: ${literal(p.value)}`);
		if (rows.length > 0) lines.push(`${pad}properties: {\n${rows.join('\n')}\n${pad}}`);
	}
	if (block.run_condition.trim()) {
		lines.push(`${pad}run_condition: ${raw(block.run_condition)}`);
	}
	if (block.kind === 'script' && block.script.trim()) {
		lines.push(`${pad}script: ${raw(block.script)}`);
	}
	if (block.speechChannel.trim()) {
		lines.push(`${pad}speech_channel: ${raw(block.speechChannel.trim())}`);
	}
	lines.push(`${pad}template_id: ${idField(block.kind === 'script' ? null : block.template)}`);
	if (block.tools.length > 0) {
		const rows = block.tools.map((t) => {
			const channel = t.eventChannel.trim();
			return `${indent(depth + 2)}{ tool_id: ${idField(t.tool)}, event_channel: ${
				channel ? raw(channel) : 'null'
			} }`;
		});
		lines.push(`${pad}tools: [\n${rows.join('\n')}\n${pad}]`);
	}
	lines.push(`${pad}user_id: null`);
	if (block.widgets.length > 0) {
		const rows = block.widgets.map((w) => `${indent(depth + 2)}${idField(w)}`);
		lines.push(`${pad}widgets: [\n${rows.join('\n')}\n${pad}]`);
	}

	return `${indent(depth)}{\n${lines.join('\n')}\n${indent(depth)}}`;
}

function castEntry(entry: CastEntry, depth: number): string {
	if (entry.kind === 'agent') return agent(entry.agent, depth);

	const pad = indent(depth);
	const body = entry.body.map((a) => agent(a, depth + 1)).join('\n');
	return [
		`${pad}{# foreach ${entry.varName} in {{input:${entry.over}}} #}`,
		body,
		`${pad}{# endforeach #}`
	].join('\n');
}

function inputsBlock(inputs: InputDecl[]): string {
	if (inputs.length === 0) return '';
	const lines = inputs.map((decl) => {
		const head = `${decl.name}: ${declaredText(decl.ty)}`;
		if (decl.defaultValue === null) return head;
		return `${head} = ${defaultLiteral(decl.ty, decl.defaultValue)}`;
	});
	return `#inputs\n${lines.join('\n')}\n\n`;
}

/** A default as weaver source. */
function defaultLiteral(ty: InputType, value: string): string {
	const base = baseOf(ty);
	if (isList(ty)) return raw(value);
	const bare = base.type === 'number' || base.type === 'bool' || base.type === 'range';
	return bare ? value : raw(value);
}

export interface Document {
	source: string;
	/** True once anything is unbound: the document is a blueprint. */
	hasInputs: boolean;
}

/** The realm's front matter, alongside its cast. */
export interface DocumentHead {
	name: string;
	description: string;
	tags: string[];
	director: Director | null;
	scenarios: ScenarioBlock[];
}

export function toWtn(head: DocumentHead, inputs: InputDecl[], cast: CastEntry[]): Document {
	const agents =
		cast.length === 0 ? '[]' : `[\n${cast.map((e) => castEntry(e, 2)).join('\n')}\n  ]`;

	const body = [
		'{',
		`${INDENT}name: ` + text(head.name),
		`${INDENT}description: ` + text(head.description),
		`${INDENT}tags: ${list(head.tags.map(raw), 1)}`,
		`${INDENT}director: ` + director(head.director),
		`${INDENT}scenarios: ${scenarios(head.scenarios)}`,
		`${INDENT}agents: ${agents}`,
		'}'
	].join('\n');

	return { source: `${inputsBlock(inputs)}${body}\n`, hasInputs: inputs.length > 0 };
}

function scenarios(items: ScenarioBlock[]): string {
	if (items.length === 0) return '[]';
	const rows = items.map((s) => {
		const guarded = hasInterpolation(s.name);
		const depth = guarded ? 3 : 2;

		const lines = [
			`${indent(depth + 1)}id: ${raw(s.id)}`,
			`${indent(depth + 1)}name: ${text(s.name)}`,
			`${indent(depth + 1)}description: ${text(s.description)}`
		];
		if (s.opening.trim()) lines.push(`${indent(depth + 1)}opening: ${text(s.opening)}`);
		const body = `${indent(depth)}{\n${lines.join('\n')}\n${indent(depth)}}`;

		if (!guarded) return body;
		return [
			`${indent(2)}{# if ${interpolated(s.name)} != ${raw('')} #}`,
			body,
			`${indent(2)}{# endif #}`
		].join('\n');
	});
	return `[\n${rows.join('\n')}\n${indent(1)}]`;
}

function list(items: string[], depth: number): string {
	if (items.length === 0) return '[]';
	const inner = items.map((item) => `${indent(depth + 1)}${item}`).join('\n');
	return `[\n${inner}\n${indent(depth)}]`;
}

function director(value: Director | null): string {
	if (value === null) return 'null';
	const pad = indent(2);
	if (value.kind === 'script') {
		return `{\n${pad}kind: ${raw('script')}\n${pad}source: ${raw(value.source)}\n${indent(1)}}`;
	}
	return [
		'{',
		`${pad}kind: ${raw(value.kind)}`,
		`${pad}over: ${filter(value.over)}`,
		`${indent(1)}}`
	].join('\n');
}

function filter(value: AgentFilter): string {
	switch (value.match) {
		case 'kind':
			return `{ match: ${raw('kind')}, kind: ${raw(value.kind)} }`;
		case 'handles':
			return `{ match: ${raw('handles')}, handles: ${list(value.handles.map(raw), 2)} }`;
		default:
			return `{ match: ${raw('all')} }`;
	}
}
