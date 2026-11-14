import { characters } from './characters.js';
import { chat } from './chat.js';
import { commands } from './commands.js';
import { common } from './common.js';
import { components } from './components.js';
import { connections } from './connections.js';
import { errors } from './errors.js';
import { inspector } from './inspector.js';
import { keys } from './keys.js';
import { nav } from './nav.js';
import { plugins } from './plugins.js';
import { realms } from './realms.js';
import { settings } from './settings.js';
import { shell } from './shell.js';
import { studio } from './studio.js';
import { transfer } from './transfer.js';
import { weaver } from './weaver.js';

/** English is the source catalog: every other locale is typed against it. */
export const en = {
	characters,
	chat,
	commands,
	common,
	components,
	connections,
	errors,
	inspector,
	keys,
	nav,
	plugins,
	realms,
	settings,
	shell,
	studio,
	transfer,
	weaver
} as const;
