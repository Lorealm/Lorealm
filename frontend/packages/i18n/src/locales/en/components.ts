/** Shared building blocks used across screens. */
export const components = {
	toast: {
		dismiss: 'Dismiss toast'
	},
	saveIndicator: {
		invalid: 'Unsaved — invalid'
	},
	wideScreenNote: (subject: string) => `${subject} is easier to use on a tablet or computer.`,
	tokenBadge: {
		estimated: 'Estimated — no tokenizer is active. Pick one in Settings for exact counts.',
		exact: 'Tokens under the active tokenizer',
		unit: 'tokens'
	},
	searchSelect: {
		noMatches: 'No matches.',
		hideSuggestions: 'Hide suggestions',
		showSuggestions: 'Show suggestions'
	},
	iconPicker: {
		change: 'Change icon',
		pick: 'Pick an icon',
		search: (count: number) => `Search ${count} icons`,
		noMatch: (query: string) => `No icon matches “${query}”.`
	},
	listSidebar: {
		new: 'New',
		creating: 'Creating…',
		search: 'Search…',
		sort: (title: string) => `Sort ${title.toLowerCase()}`,
		showBuiltIn: 'Show built-in'
	},
	field: {
		help: (label: string) => `Help: ${label}`,
		maximize: 'Maximize',
		maximizeField: (label: string) => `Maximize ${label}`,
		fallbackLabel: 'field',
		editingFullScreen: 'Editing full screen…',
		edit: 'Edit'
	},
	numberInput: {
		increment: 'Increment',
		decrement: 'Decrement'
	},
	plugin: {
		callFailed: 'The call failed.',
		panelFailed: "This plugin's panel couldn't be loaded."
	},
	loading: 'Loading',
	unknownWidget: (id: string) => `Unknown widget: ${id}`,
	maximize: {
		minimizeHint: 'Minimize (Esc)',
		minimize: 'Minimize'
	}
} as const;
