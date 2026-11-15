export { default as App } from './App.svelte';
export type {
	PlatformAPI,
	AuthApi,
	FileDialog,
	FileTransfer,
	HttpTransport,
	LocalStore,
	ServerControls,
	UpdateControls,
	AvailableUpdate
} from './core/api.ts';
export type { User } from './types/user.js';
export type { Session } from './types/auth.ts';
export { extractError, errorDomain } from './types/error.js';
export type {
	ErrorCode,
	ErrorDomain,
	ErrorDetails,
	ExtractedError,
	ResourceKind
} from './types/error.js';

export {
	builtInSlashCommands,
	dispatch,
	parseArgs,
	parseInput,
	serverCommands,
	slashCommands,
	usage
} from './commands/index.js';
export type {
	CommandArgs,
	ParsedInput,
	RegisteredSlashCommand,
	SlashCommand,
	SlashParam
} from './commands/index.js';

export {
	builtInCommands,
	chordsFromEvent,
	commandRegistry,
	formatChord,
	GLOBAL_LAYER,
	inputEnvironment,
	IS_MAC,
	keybindConfig,
	keybinds,
	keyLayer,
	layers,
	parseChord
} from './keys/index.js';
export type {
	Chord,
	Command,
	CommandContext,
	CommandResult,
	CommandSource,
	Conflict,
	KeybindMode,
	KeybindSettings,
	Layer,
	LayerOptions,
	LayerScope,
	RegisteredCommand
} from './keys/index.js';
