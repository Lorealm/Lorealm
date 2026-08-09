import type { Snowflake } from './core.ts';

export type StatePool = Record<string, Record<string, unknown>>;

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function readPath(namespace: Record<string, unknown> | undefined, name: string): unknown {
	if (!namespace) return undefined;
	const [root, ...rest] = name.split('.');
	if (!Object.prototype.hasOwnProperty.call(namespace, root)) return undefined;

	let value = namespace[root];
	for (const segment of rest) {
		if (!isRecord(value) || !Object.prototype.hasOwnProperty.call(value, segment)) {
			return undefined;
		}
		value = value[segment];
	}
	return value;
}

export function flattenPaths(namespace: Record<string, unknown>): Array<{
	key: string;
	value: unknown;
}> {
	const out: Array<{ key: string; value: unknown }> = [];
	const walk = (prefix: string, value: unknown) => {
		if (isRecord(value) && Object.keys(value).length > 0) {
			for (const [k, v] of Object.entries(value)) walk(prefix ? `${prefix}.${k}` : k, v);
			return;
		}
		out.push({ key: prefix, value });
	};
	for (const [k, v] of Object.entries(namespace)) walk(k, v);
	return out.sort((a, b) => a.key.localeCompare(b.key));
}

export interface ReconstructedState {
	persistent: StatePool;
	activation: Record<Snowflake, ActivationState>;
}

export interface ActivationState {
	last_activated: Record<string, number>;
	sticky_remaining: Record<string, number>;
	current_turn: number;
}
