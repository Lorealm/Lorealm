import type { Component } from 'svelte';

type Loader<P extends Record<string, unknown>> = () => Promise<{ default: Component<P> }>;

const cache = new WeakMap<Loader<never>, Promise<Component<never>>>();

export function lazy<P extends Record<string, unknown>>(load: Loader<P>): Promise<Component<P>> {
	let pending = cache.get(load as Loader<never>) as Promise<Component<P>> | undefined;
	if (!pending) {
		pending = load().then((module) => module.default);
		cache.set(load as Loader<never>, pending as Promise<Component<never>>);
	}
	return pending;
}
