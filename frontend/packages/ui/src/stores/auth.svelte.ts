import type { Session } from '$lib/types/auth.js';

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
}
