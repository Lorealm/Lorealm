export const shell = {
	login: {
		title: 'Login Page',
		serverUrl: 'Server URL',
		username: 'Username',
		password: 'Password',
		emailOptional: 'Email (optional)',
		inviteCode: 'Invite code',
		invitePlaceholder: 'Needed if sign-ups are closed',
		createAccount: 'Create account',
		haveAccount: 'I already have an account',
		logIn: 'Log in',
		createAnAccount: 'Create an account'
	},
	closePanel: 'Close panel',
	window: {
		minimize: 'Minimize',
		maximize: 'Maximize',
		close: 'Close'
	},
	underConstruction: 'Under Construction',
	update: {
		title: 'Update available',
		message: (version: string) => `Lorealm ${version} is ready to install.`,
		install: 'Restart and update',
		later: 'Later',
		installingTitle: 'Updating',
		installing: 'Downloading the update. Lorealm will restart when it’s done.',
		failed: 'The update couldn’t be installed'
	}
} as const;
