import type { Snowflake } from './core.js';

/**
 * Permission flags. Mirrors `Permissions` in
 * `backend/server/src/services/permission/mod.rs`. Bits travel as decimal
 * strings because the high flags don't fit in a JavaScript number.
 */
export const Permission = {
	IMPORT_CONTENT: 1n << 0n,
	UPLOAD_TOKENIZERS: 1n << 1n,
	PUBLISH_PUBLIC: 1n << 2n,
	CUSTOM_ENDPOINTS: 1n << 3n,
	USE_PLUGINS: 1n << 16n,
	INSTALL_PLUGINS: 1n << 17n,
	BUILD_PLUGINS: 1n << 18n,
	UNSAFE_PLUGINS: 1n << 19n,
	MANAGE_USERS: 1n << 32n,
	MANAGE_ROLES: 1n << 33n,
	MANAGE_SERVER: 1n << 34n,
	ADMINISTRATOR: 1n << 62n
} as const;

/** Flags that let third-party code run on the server or in the browser. */
export const CODE_EXECUTION = Permission.BUILD_PLUGINS | Permission.UNSAFE_PLUGINS;

export interface PermissionInfo {
	flag: bigint;
	label: string;
	hint: string;
}

export const PERMISSION_GROUPS: Array<{ title: string; items: PermissionInfo[] }> = [
	{
		title: 'Content',
		items: [
			{
				flag: Permission.IMPORT_CONTENT,
				label: 'Import content',
				hint: 'Import character cards, realms and other outside formats.'
			},
			{
				flag: Permission.UPLOAD_TOKENIZERS,
				label: 'Upload tokenizers',
				hint: 'Upload tokenizers or have the server download them.'
			},
			{
				flag: Permission.PUBLISH_PUBLIC,
				label: 'Publish publicly',
				hint: 'Make things, such as a tokenizer, visible to every user.'
			},
			{
				flag: Permission.CUSTOM_ENDPOINTS,
				label: 'Custom endpoints',
				hint: 'Point connections at any URL, including ones on the server’s own network.'
			}
		]
	},
	{
		title: 'Plugins',
		items: [
			{
				flag: Permission.USE_PLUGINS,
				label: 'Use plugins',
				hint: 'Enable, configure and use plugins installed on the server.'
			},
			{
				flag: Permission.INSTALL_PLUGINS,
				label: 'Install plugins',
				hint: 'Install and uninstall pre-built plugins for everyone.'
			},
			{
				flag: Permission.BUILD_PLUGINS,
				label: 'Build plugins',
				hint: 'Build plugins from source, which runs their code on the server. Administrator doesn’t include this.'
			},
			{
				flag: Permission.UNSAFE_PLUGINS,
				label: 'Unsafe plugins',
				hint: 'Run plugins with full access to the app in your browser. Administrator doesn’t include this.'
			}
		]
	},
	{
		title: 'Administration',
		items: [
			{ flag: Permission.MANAGE_USERS, label: 'Manage users', hint: 'Manage user accounts.' },
			{
				flag: Permission.MANAGE_ROLES,
				label: 'Manage roles',
				hint: 'Create and edit roles below your own, and give them to people.'
			},
			{
				flag: Permission.MANAGE_SERVER,
				label: 'Manage server',
				hint: 'Change server settings and see them in full.'
			},
			{
				flag: Permission.ADMINISTRATOR,
				label: 'Administrator',
				hint: 'Everything except building and unsafe plugins.'
			}
		]
	}
];

export interface Role {
	id: Snowflake;
	name: string;
	/** Decimal string of permission bits. */
	permissions: string;
	position: number;
	created_at: string;
}

export interface CreateRoleRequest {
	name: string;
	permissions: string;
	position: number;
}

export interface UpdateRoleRequest {
	name?: string;
	permissions?: string;
	position?: number;
}

export interface MyPermissions {
	/** Decimal string of effective permission bits, Administrator expanded. */
	permissions: string;
	/** You can manage roles positioned below this. */
	highest_position: number;
}

/** Built-in roles, seeded by migration. */
export const EVERYONE_ROLE_ID = '1';
export const ADMINISTRATOR_ROLE_ID = '2';
