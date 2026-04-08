import type { Session } from '$lib/types/auth.js';
import { ActiveView, navigationStore } from './navigation.svelte.js';

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
	navigationStore.setAuthState(state);
	if (state.status == 'authenticated') {
		navigationStore.setView(ActiveView.Main);
	}
}
