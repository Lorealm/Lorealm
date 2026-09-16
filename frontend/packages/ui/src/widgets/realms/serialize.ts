import type { AgentBlock, Binding, CastEntry, InputDecl, InputType } from './model.js';
import { baseOf, declaredText, isList } from './model.js';

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
	switch (binding.source) {
		case 'resource':
			return raw(binding.id);
		case 'loop':
			return `{{${binding.varName}.id}}`;
		case 'input':
			return `{{input:${binding.name}.id}}`;
	}
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
	if (block.run_condition.trim()) {
		lines.push(`${pad}run_condition: ${raw(block.run_condition)}`);
	}
	if (block.kind === 'script' && block.script.trim()) {
		lines.push(`${pad}script: ${raw(block.script)}`);
	}
	lines.push(`${pad}template_id: ${idField(block.kind === 'script' ? null : block.template)}`);
	lines.push(`${pad}user_id: null`);

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
	return base.type === 'number' || base.type === 'bool' ? value : raw(value);
}

export interface Document {
	source: string;
	/** True once anything is unbound: the document is a blueprint. */
	hasInputs: boolean;
}

export function toWtn(
	name: string,
	description: string,
	inputs: InputDecl[],
	cast: CastEntry[]
): Document {
	const agents =
		cast.length === 0 ? '[]' : `[\n${cast.map((e) => castEntry(e, 2)).join('\n')}\n  ]`;

	const body = [
		'{',
		`${INDENT}agents: ${agents}`,
		`${INDENT}description: ` + text(description),
		`${INDENT}director: null`,
		`${INDENT}name: ` + text(name),
		`${INDENT}scenarios: []`,
		'}'
	].join('\n');

	return { source: `${inputsBlock(inputs)}${body}\n`, hasInputs: inputs.length > 0 };
}
