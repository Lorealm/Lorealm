import { plural } from './plural.js';

export const chat = {
	feed: {
		nothingYet: 'Nothing has happened yet.',
		pickCharacter: 'Pick a character and send a message to begin.',
		worldStirs: 'the world stirs…'
	},
	turns: {
		running: 'Running turn…',
		yourTurnAs: 'Your turn as',
		emptyRunFirst: 'Nothing has happened yet — run the first turn.',
		nextToAct: 'Next to act:',
		nobody: 'Nobody can act right now.',
		seeingAll: 'Seeing every turn',
		seeingOwn: 'Seeing only what your agent perceives',
		director: 'Director',
		player: 'Player',
		pauseHint: 'Finish this turn, then stop',
		pause: 'Pause',
		cancelHint: 'Cancel this turn',
		stop: 'Stop',
		stepHint: 'Run a single turn',
		step: 'Step',
		runHint: "Run turns until it's your turn",
		run: 'Run'
	},
	composer: {
		functions: 'Functions',
		conversations: 'Conversations',
		newChat: 'Start new chat',
		attachments: 'Attachments',
		placeholder: 'Send a message...',
		send: 'Send',
		stopCommand: (name: string) => `Stop /${name}`,
		stopPlugin: (name: string) => `Stop ${name}`,
		stopGenerating: 'Stop generating',
		commands: 'Commands'
	},
	message: {
		you: 'You',
		system: 'System',
		assistant: 'Assistant',
		ranCommand: (name: string) => `Ran /${name}`,
		inspectContext: 'Inspect context',
		continue: 'Continue this reply',
		thoughts: 'Thoughts',
		thinking: 'Thinking…',
		swipe: 'Swipe'
	},
	setup: {
		title: 'Welcome to Lorealm',
		lead: 'You need to complete a few steps before you can get started:',
		done: '(done)',
		todo: '(to do)',
		noProfile: 'Create a profile and star it as your active connection.',
		usingProfile: (name: string) => `Using ${name}.`,
		yourProfileLower: 'your profile',
		yourProfile: 'Your profile',
		needsKey: (profile: string, provider: string) =>
			`${profile} needs an API key to reach ${provider}.`,
		template: {
			title: 'Create and enable a Context Template',
			hint: 'This is how the text sent to the model is laid out. Without it there is nothing to send.'
		},
		persona: {
			title: 'Create and enable a Persona',
			hint: 'A persona is the details about your character. Create and enable one in the studio.'
		},
		connection: {
			title: 'Configure your connection profile'
		},
		installTokenizer: {
			title: 'Install a tokenizer',
			hint: 'Import one from a file or download it, so messages can be measured against your budget.'
		},
		activeTokenizer: {
			title: 'Set your active tokenizer',
			hint: 'Pick which installed tokenizer counts your tokens.'
		}
	},
	inspector: {
		title: 'Context Inspector',
		dialog: 'Context inspector',
		tokenCountFailed: 'Token count failed',
		stateFailed: 'Failed to load state',
		sampling: {
			temperature: 'Temperature',
			topP: 'Top P',
			topK: 'Top K',
			minP: 'Min P'
		},
		unknownTool: '(unknown)',
		untitledNode: '(untitled node)',
		node: (id: string) => `node ${id}`,
		notReconstructed: 'content not reconstructed',
		noTrace: 'no trace on this message',
		blocks: (count: number) => plural(count, { one: `${count} block`, other: `${count} blocks` }),
		units: { tokens: 'tokens', chars: 'chars' },
		generationSettings: 'Generation settings',
		tokenBudget: 'Token budget',
		unlimited: 'unlimited',
		profileFailed: "couldn't load the connection profile",
		fromProfile: (name: string) => `from ${name}`,
		budgetNotRecorded:
			"Token budget wasn't recorded on this turn, so this is the profile's current value.",
		layout: 'Layout',
		noBlocks: 'no blocks',
		layoutAria: 'Context layout, oldest at top',
		nodeFallback: 'node',
		blockAria: (role: string, size: string) => `${role} block, ${size}`,
		roles: { system: 'system', user: 'user', assistant: 'assistant' },
		bySource: 'By source',
		drift: (count: string) =>
			`Recorded ${count} tokens when this turn was sent. The difference is content edited since — the panel recounts what those blocks say now.`,
		was: (count: string) => `was ${count}`,
		trimmed: 'Trimmed to fit',
		shed: (source: string, count: string) => `${source} · ${count} tokens shed to fit the budget`,
		sources: {
			conversation: 'chat history',
			context_template: 'template',
			lorebook: 'lorebook',
			character: 'character',
			character_examples: 'example messages',
			miscellaneous: 'system prompts',
			plugin: 'plugins'
		},
		tabs: { context: 'Context', state: 'State', tools: 'Tools' },
		rawHint: 'Show the context as raw text',
		raw: 'Raw',
		noTraceLong:
			'This message carries no context trace. Only turns generated through the weaver pipeline record one.',
		noContextBlocks: 'The pipeline produced no context blocks for this turn.',
		msgs: (count: number) => `${count} msgs`,
		loadingState: 'Loading state…',
		noState: 'No reconstructed state for this turn.',
		activation: 'Activation',
		noActivation: 'no activation state',
		noTrackedEntries: 'no tracked entries',
		lastActivated: (turns: string) => `last activated ${turns} turns ago`,
		act: (turns: string) => `act ${turns}`,
		stickyRemaining: (turns: number) => `${turns} sticky turns remaining`,
		stk: (turns: string) => `stk ${turns}`,
		persistent: 'Persistent',
		noPersistent: 'no persistent state',
		pool: 'POOL',
		emptyPool: 'empty pool',
		available: 'Available',
		tools: (count: number) => plural(count, { one: `${count} tool`, other: `${count} tools` }),
		noToolRecord: "This message doesn't record which tools were available.",
		noToolsOffered: 'The model was offered no tools on this turn.',
		noToolCalls: 'No tool calls on this turn.',
		round: (n: number) => `Round ${n}`,
		calls: (count: number) => plural(count, { one: `${count} call`, other: `${count} calls` }),
		error: 'error',
		ok: 'ok',
		input: 'input',
		output: 'output',
		refs: {
			definition: 'character · definition',
			personality: 'character · personality',
			scenario: 'character · scenario',
			greeting: (n: number) => `character · greeting #${n}`,
			example: (n: number) => `character · example #${n}`,
			character: 'character',
			unresolvedMessage: 'message · unresolved',
			persona: (name: string) => `persona · ${name}`,
			plugin: (id: string) => `plugin · ${id}`
		}
	},
	conversations: {
		title: 'Conversations',
		loadFailed: 'Failed to load conversations',
		openFailed: 'Failed to open conversation',
		confirmDelete: (name: string) => `Delete "${name}"? This can't be undone.`,
		messages: (count: number) =>
			plural(count, { one: `${count} message`, other: `${count} messages` }),
		delete: 'Delete conversation',
		loading: 'Loading…',
		empty: 'No conversations with this character yet.'
	}
} as const;
