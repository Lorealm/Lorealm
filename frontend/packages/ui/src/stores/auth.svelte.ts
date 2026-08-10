import type { Session } from '$lib/types/auth.js';
import { router } from '$lib/navigation/router.svelte.js';

export type AuthState =
	| { status: 'loading' }
	| { status: 'unauthenticated' }
	| { status: 'authenticated'; session: Session };

let authState = $state<AuthState>({ status: 'loading' });

export function getAuthState() {
	return authState;
}

export function setAuthState(state: AuthState) {
	authState = state;
	if (state.status === 'authenticated') router.enter();
	else router.reset();
}
