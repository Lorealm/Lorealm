import { type AuthState } from './auth.svelte';

export enum ActiveView {
	Login,
	Main
}

export enum SidebarView {
	Write = 'Write',
	Characters = 'Characters',
	Worlds = 'Worlds',
	Studio = 'Studio',
	Settings = 'Settings'
}

let activeView = $state(ActiveView.Login);
let sidebarView = $state(SidebarView.Write);
let authState = $state<AuthState>({ status: 'loading' });

export const navigationStore = {
	setAuthState(state: AuthState) {
		authState = state;
	},

	setView(view: ActiveView) {
		console.log('New view: ', view);
		activeView = view;
	},

	setSidebarView(view: SidebarView) {
		sidebarView = view;
	},

	get active() {
		if (authState.status != 'authenticated') return ActiveView.Login;
		return activeView;
	},

	get sidebarActive() {
		return sidebarView;
	}
};
