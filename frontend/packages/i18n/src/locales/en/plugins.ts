import { plural } from './plural.js';

/** Installing, browsing and managing plugins. */
export const plugins = {
	tabs: {
		list: 'Plugins',
		installed: 'Installed',
		marketplace: 'Marketplace'
	},
	loading: 'Loading…',
	installedReport: (handle: string, version: string) => `Installed ${handle}@${version}.`,
	addManually: 'Add from a link or file',
	location: 'Link or folder on the server',
	locationHint:
		'A GitHub repository or release, a link to a .zip, a git repository, or a folder holding a manifest.toml.',
	dev: 'For development',
	devHint: 'Rebuild and reload it from its folder whenever you change it.',
	upload: 'Or upload a .zip',
	installing: 'Installing…',
	install: 'Install',
	noneInstalled: 'No plugins are installed yet.',
	browse: 'Browse the marketplace',
	updateTo: (version: string) => `Update to ${version}`,
	enabled: 'Enabled',
	enable: 'Enable',
	settingsNamed: (name: string) => `${name} settings`,
	settings: 'Settings',
	removeNamed: (name: string) => `Remove ${name} for everyone`,
	remove: 'Remove for everyone',
	notListed: "This plugin isn't in the marketplace.",
	manage: 'Manage',
	noCompatible: 'No version of it works with this version of Lorealm.',
	notOnServer: "It isn't on this server yet. Someone who manages its plugins can add it.",
	status: {
		unsupported: 'Not for this version of Lorealm',
		update: 'Update available',
		enabled: 'Enabled',
		installed: 'Installed'
	},
	search: 'Search plugins',
	sourcesMissing: (count: number) =>
		plural(count, {
			one: "Some plugins may be missing: one plugin source couldn't be reached.",
			other: `Some plugins may be missing: ${count} plugin sources couldn't be reached.`
		}),
	marketplaceFailed: "The marketplace couldn't be loaded.",
	marketplaceEmpty: 'The marketplace has no plugins yet.',
	noMatch: (query: string) => `No plugin matches “${query}”.`,
	permissions: {
		'chat:read': 'Read your chats',
		'chat:write': 'Change or refuse messages before they are saved',
		'data:persist': 'Store its own data',
		'model:generate': 'Generate a response on your behalf',
		'net:fetch': 'Send requests to the hosts below',
		'persona:read': 'Read your active persona',
		'prompt:write': 'Change prompts before they are sent',
		'state:read': 'Read script variables',
		'state:write': 'Change script variables',
		'tools:write': 'Change tool calls and their results, or refuse calls',
		'ui:autoplay': 'Play media without you clicking it first',
		'ui:unsafe': 'Run code directly in the app'
	},
	callouts: {
		note: 'Note',
		tip: 'Tip',
		important: 'Important',
		warning: 'Warning',
		caution: 'Caution'
	},
	repository: 'Repository',
	permissionsTitle: 'Permissions',
	sendRequestsTo: 'Send requests to',
	none: 'None.',
	noReadme: 'This plugin has no README.',
	notInstalled: "This plugin isn't installed.",
	reloadHint: 'Rebuild from its folder and reload',
	reload: 'Reload'
} as const;
