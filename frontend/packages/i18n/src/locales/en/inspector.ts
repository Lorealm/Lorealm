import { plural } from './plural.js';

/** The side panel next to the main view. */
export const inspector = {
	resize: (width: number) => `Resize inspector, currently ${width} pixels. Use arrow keys.`,
	realmPlaceholder: 'Realm Placeholder',
	close: 'Close inspector',
	nothingConfigured: 'Nothing configured for this view.',
	widgets: {
		characterImage: 'Character Image',
		template: 'Template',
		context: 'Context',
		state: 'State',
		realmAgent: 'Realm Agent',
		authorNote: "Author's Note",
		creatorNotes: "Creator's Notes"
	},
	creatorNotes: {
		loading: 'Loading…',
		noCharacter: 'No active character. Pick one on the characters screen to read its notes.',
		empty: "This character has no creator's notes."
	},
	authorNote: {
		noChat: 'No active chat.',
		placeholder: '{{char:name}} has achieved something remarkable...'
	},
	image: {
		cover: 'Fill the panel, cropping the edges',
		fit: 'Fit the whole image inside the panel',
		full: 'Grow the panel to the whole image',
		change: 'Change how the image fits',
		noCharacter: 'No active character.'
	},
	context: {
		title: 'Live Context',
		blocks: (count: number) => plural(count, { one: `${count} block`, other: `${count} blocks` }),
		noTrace: 'No traced turn yet.',
		noBlocks: 'Latest turn produced no blocks.',
		kinds: { source: 'SRC', group: 'GRP', text: 'TXT' },
		untitled: '(untitled)',
		source: (id: string) => `source ${id}`
	},
	template: {
		title: 'Template Nodes',
		toggleFailed: 'Toggle failed',
		rowHint: 'Click to toggle, shift-click to collapse',
		boxHint: 'Click to toggle, shift-click to open',
		untitled: 'Untitled',
		showGrid: 'Show as grid',
		showTree: 'Show as tree',
		noTemplate: 'No active template.',
		nodes: 'Template nodes',
		empty: 'Empty template.',
		emptyGroup: 'Empty group.'
	},
	state: {
		title: 'Live State',
		turn: (n: number) => `turn ${n}`,
		noMessage: 'No message pinned.',
		unavailable: 'State unavailable.',
		noActivation: 'No activation state.',
		unknown: 'Unknown',
		noPersistent: 'No persistent state.'
	}
} as const;
