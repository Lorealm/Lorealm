import type { Messages } from '@lorealm/i18n';
import { t } from '#lib/stores/i18n.svelte.js';
import type { Snowflake } from './core.js';

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

type PermissionKey = keyof Messages['settings']['permissions']['flags'];

/** Label and hint follow the active locale. */
function info(flag: bigint, key: PermissionKey): PermissionInfo {
	return {
		flag,
		get label() {
			return t.settings.permissions.flags[key].label;
		},
		get hint() {
			return t.settings.permissions.flags[key].hint;
		}
	};
}

export const PERMISSION_GROUPS: Array<{ title: string; items: PermissionInfo[] }> = [
	{
		get title() {
			return t.settings.permissions.groups.content;
		},
		items: [
			info(Permission.IMPORT_CONTENT, 'importContent'),
			info(Permission.UPLOAD_TOKENIZERS, 'uploadTokenizers'),
			info(Permission.PUBLISH_PUBLIC, 'publishPublic'),
			info(Permission.CUSTOM_ENDPOINTS, 'customEndpoints')
		]
	},
	{
		get title() {
			return t.settings.permissions.groups.plugins;
		},
		items: [
			info(Permission.USE_PLUGINS, 'usePlugins'),
			info(Permission.INSTALL_PLUGINS, 'installPlugins'),
			info(Permission.BUILD_PLUGINS, 'buildPlugins'),
			info(Permission.UNSAFE_PLUGINS, 'unsafePlugins')
		]
	},
	{
		get title() {
			return t.settings.permissions.groups.administration;
		},
		items: [
			info(Permission.MANAGE_USERS, 'manageUsers'),
			info(Permission.MANAGE_ROLES, 'manageRoles'),
			info(Permission.MANAGE_SERVER, 'manageServer'),
			info(Permission.ADMINISTRATOR, 'administrator')
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
