import {
	variable,
	type VariableRef,
	type VariableSource
} from '$lib/weaver/editor/weaver-variables.js';
import { realmDraft } from './draft.svelte.js';
import { declaredText, isRequired, type AgentBlock } from './model.js';

export function realmVariables(agentKey?: string): VariableSource {
	return { variables: () => collect(agentKey), elements: castElements };
}

const CAST_MEMBERS = /^\{\{-?\s*cast:(?:(llm|player|script)\.)?members\s*-?\}\}$/;

function castElements(iterable: string): VariableRef[] | null {
	const match = CAST_MEMBERS.exec(iterable.trim());
	if (!match) return null;

	const kind = match[1];
	const walked = everyAgent().filter((a) => a.handle && (!kind || a.kind === kind));

	const fields = [
		variable('', 'handle', 'cast member', 'This member’s handle.', 'string'),
		variable('', 'name', 'cast member', 'This member’s display name.', 'string'),
		variable('', 'kind', 'cast member', '`llm`, `player` or `script`.', 'string'),
		variable('', 'turn_order', 'cast member', 'Its place in the rotation.', 'number')
	];

	const carriers = new Map<string, string[]>();
	for (const agent of walked) {
		for (const property of agent.properties) {
			if (!property.key.trim()) continue;
			carriers.set(property.key, [...(carriers.get(property.key) ?? []), agent.handle]);
		}
	}
	for (const [key, handles] of carriers) {
		const all = handles.length === walked.length;
		fields.push(
			variable(
				'',
				key,
				all ? 'property' : `property · ${handles.length}/${walked.length}`,
				all
					? 'Carried by every member this loop walks.'
					: `Carried by ${handles.map((h) => `\`${h}\``).join(', ')}. Reads as nothing on the rest.`
			)
		);
	}
	return fields;
}

function everyAgent(): AgentBlock[] {
	return realmDraft.cast.flatMap((entry) => (entry.kind === 'agent' ? [entry.agent] : entry.body));
}

function grantName(scope: string): string | null {
	const name = scope.trim().replace(/\.?\*$/, '');
	return name === '' ? null : name;
}

/** The declared inputs, for a view of the document itself. */
export function inputVariables(): VariableSource {
	return {
		variables: () =>
			realmDraft.inputs.map((decl) =>
				variable(
					'input',
					decl.name,
					isRequired(decl) ? 'input' : 'optional input',
					decl.doc.trim() || 'Chosen by whoever builds a realm from this.',
					declaredText(decl.ty)
				)
			)
	};
}

function collect(agentKey?: string): VariableRef[] {
	const refs: VariableRef[] = [];
	const own = agentKey ? realmDraft.agent(agentKey) : null;

	for (const agent of everyAgent()) {
		if (!agent.handle) continue;
		refs.push(
			variable(
				'cast',
				`agents.${agent.handle}.name`,
				'cast member',
				`${agent.name || agent.handle}'s display name.`,
				'string'
			),
			variable('cast', `agents.${agent.handle}.kind`, 'cast member', agent.kind, 'string'),
			variable(
				'cast',
				`agents.${agent.handle}.turn_order`,
				'cast member',
				'Where this member sits in the rotation.',
				'number'
			)
		);
		for (const property of agent.properties) {
			if (!property.key.trim()) continue;
			const seeded = `Seeded to \`${property.value}\` on ${agent.handle}.`;
			refs.push(
				variable('agents', `${agent.handle}.${property.key}`, 'property', seeded),
				variable('cast', `agents.${agent.handle}.${property.key}`, 'property', seeded)
			);
		}
	}

	for (const property of own?.properties ?? []) {
		if (!property.key.trim()) continue;
		refs.push(variable('self', property.key, 'own property', `Seeded to \`${property.value}\`.`));
	}

	for (const grant of own?.grants ?? []) {
		if (!grant.namespace.trim()) continue;
		refs.push(
			variable(
				grant.namespace,
				grantName(grant.scope) ?? '',
				`granted ${grant.access}`,
				`State this agent may ${grant.access === 'read' ? 'read' : 'read and write'}.`
			)
		);
	}

	return refs;
}
