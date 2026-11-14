export const keys = {
	groups: {
		navigation: 'Navigation',
		views: 'Views',
		settings: 'Settings',
		chat: 'Chat',
		inspector: 'Inspector'
	},
	commands: {
		navBack: { title: 'Back', description: 'Return to the previous destination.' },
		navForward: { title: 'Forward', description: 'Undo a Back.' },
		viewWrite: { title: 'Go to Write' },
		viewCharacters: { title: 'Go to Characters' },
		viewRealms: { title: 'Go to Realms' },
		viewConnections: { title: 'Go to Connections' },
		viewStudio: { title: 'Go to Studio' },
		settingsOpen: { title: 'Open settings' },
		keysHelp: { title: 'Keyboard shortcuts', description: 'Open the shortcut list.' },
		chatFocus: { title: 'Focus the composer', description: 'Put the cursor in the chat box.' },
		chatSend: { title: 'Send message' },
		chatConversations: {
			title: 'Conversations',
			description: 'List the chats with the active character.'
		},
		chatConversationsClose: { title: 'Close conversations' },
		chatBranchPrev: {
			title: 'Previous branch',
			description: 'Swipe the newest message back to its previous variant.'
		},
		chatBranchNext: {
			title: 'Next branch',
			description:
				'Swipe the newest message forward, generating a fresh reply once past the last variant.'
		},
		inspectorNextProfile: { title: 'Next inspector profile' },
		inspectorPrevProfile: { title: 'Previous inspector profile' }
	},
	/** Key names shown in shortcuts on Windows and Linux. macOS uses symbols. */
	names: {
		ctrl: 'Ctrl',
		alt: 'Alt',
		shift: 'Shift',
		win: 'Win',
		enter: 'Enter',
		backspace: 'Backspace',
		delete: 'Delete',
		escape: 'Esc',
		tab: 'Tab',
		space: 'Space'
	}
} as const;
