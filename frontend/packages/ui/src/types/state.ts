import type { Snowflake } from './core.ts';

export type StatePool = Record<string, Record<string, unknown>>;

export interface ReconstructedState {
	persistent: StatePool;
	activation: Record<Snowflake, ActivationState>;
}

export interface ActivationState {
	last_activated: Record<string, number>;
	sticky_remaining: Record<string, number>;
	current_turn: number;
}
