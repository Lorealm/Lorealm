import {
	variable,
	type VariableRef,
	type VariableSource
} from '#lib/weaver/editor/weaver-variables.js';
import { realmDraft } from './draft.svelte.js';
import { declaredText, isRequired, type AgentBlock } from './model.js';
import { t } from '#lib/stores/i18n.svelte.js';

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
		variable('', 'handle', t.realms.variables.castMember, t.realms.variables.handle, 'string'),
		variable('', 'name', t.realms.variables.castMember, t.realms.variables.name, 'string'),
		variable('', 'kind', t.realms.variables.castMember, t.realms.variables.kind, 'string'),
		variable(
			'',
			'turn_order',
			t.realms.variables.castMember,
			t.realms.variables.turnOrder,
			'number'
		)
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
				all
					? t.realms.variables.property
					: t.realms.variables.propertyShare(handles.length, walked.length),
				all
					? t.realms.variables.carriedByAll
					: t.realms.variables.carriedBy(handles.map((h) => `\`${h}\``).join(', '))
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
					isRequired(decl) ? t.realms.variables.input : t.realms.variables.optionalInput,
					decl.doc.trim() || t.realms.variables.inputDoc,
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
				t.realms.variables.castMember,
				t.realms.variables.agentName(agent.name || agent.handle),
				'string'
			),
			variable(
				'cast',
				`agents.${agent.handle}.kind`,
				t.realms.variables.castMember,
				agent.kind,
				'string'
			),
			variable(
				'cast',
				`agents.${agent.handle}.turn_order`,
				t.realms.variables.castMember,
				t.realms.variables.seat,
				'number'
			)
		);
		for (const property of agent.properties) {
			if (!property.key.trim()) continue;
			const seeded = t.realms.variables.seededOn(property.value, agent.handle);
			refs.push(
				variable('agents', `${agent.handle}.${property.key}`, t.realms.variables.property, seeded),
				variable(
					'cast',
					`agents.${agent.handle}.${property.key}`,
					t.realms.variables.property,
					seeded
				)
			);
		}
	}

	for (const property of own?.properties ?? []) {
		if (!property.key.trim()) continue;
		refs.push(
			variable(
				'self',
				property.key,
				t.realms.variables.ownProperty,
				t.realms.variables.seeded(property.value)
			)
		);
	}

	for (const grant of own?.grants ?? []) {
		if (!grant.namespace.trim()) continue;
		refs.push(
			variable(
				grant.namespace,
				grantName(grant.scope) ?? '',
				t.realms.variables.granted(grant.access),
				grant.access === 'read' ? t.realms.variables.mayRead : t.realms.variables.mayReadWrite
			)
		);
	}

	return refs;
}
