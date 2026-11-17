import type { Messages } from '@lorealm/i18n';
import { t } from '#lib/stores/i18n.svelte.js';

/** How a value is edited. Anything unclassified falls back to a text field. */
export type TokenKind = 'color' | 'px' | 'font' | 'text';

export interface TokenField {
	/** The custom property this row writes. */
	name: string;
	label: string;
	kind: TokenKind;
}

export interface TokenGroup {
	title: string;
	blurb: string;
	fields: TokenField[];
}

/** The families the client bundles. */
export const FONT_CHOICES: Array<{ label: string; value: string }> = [
	{ label: 'Quicksand', value: 'var(--font-family-quicksand)' },
	{ label: 'Nunito', value: 'var(--font-family-nunito)' },
	{ label: 'Josefin Sans', value: 'var(--font-family-josefin)' },
	{ label: 'Karla', value: 'var(--font-family-karla)' },
	{
		get label() {
			return t.settings.interface.fonts.system;
		},
		value: 'var(--font-family-system)'
	},
	{
		get label() {
			return t.settings.interface.fonts.monospace;
		},
		value: 'var(--font-family-mono)'
	}
];

type TokenName = keyof Messages['settings']['interface']['tokens'];

/** A row whose label follows the active locale. */
const field = (name: TokenName, kind: TokenKind): TokenField => ({
	name,
	get label() {
		return t.settings.interface.tokens[name];
	},
	kind
});

const colors = (names: TokenName[]): TokenField[] => names.map((name) => field(name, 'color'));

const lengths = (names: TokenName[]): TokenField[] => names.map((name) => field(name, 'px'));

export const APPEARANCE_GROUPS: TokenGroup[] = [
	{
		get title() {
			return t.settings.interface.groups.accent.title;
		},
		get blurb() {
			return t.settings.interface.groups.accent.blurb;
		},
		fields: colors([
			'--color-primary',
			'--color-primary-light',
			'--color-primary-lighter',
			'--color-primary-dark',
			'--color-primary-darker'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.surfaces.title;
		},
		get blurb() {
			return t.settings.interface.groups.surfaces.blurb;
		},
		fields: colors([
			'--color-bg-base',
			'--color-bg-elevated',
			'--color-bg-elevated-2',
			'--color-bg-elevated-3',
			'--color-bg-panel-solid',
			'--color-bg-input',
			'--color-bg-modal',
			'--color-bg-disabled'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.text.title;
		},
		get blurb() {
			return t.settings.interface.groups.text.blurb;
		},
		fields: colors([
			'--color-text-primary',
			'--color-text-secondary',
			'--color-text-muted',
			'--color-text-disabled',
			'--color-text-accent',
			'--color-text-inverse'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.borders.title;
		},
		get blurb() {
			return t.settings.interface.groups.borders.blurb;
		},
		fields: colors([
			'--color-border-subtle',
			'--color-border-medium',
			'--color-border-strong',
			'--color-border-active',
			'--color-border-default'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.interaction.title;
		},
		get blurb() {
			return t.settings.interface.groups.interaction.blurb;
		},
		fields: colors(['--color-hover', '--color-active', '--color-focus', '--color-selected'])
	},
	{
		get title() {
			return t.settings.interface.groups.status.title;
		},
		get blurb() {
			return t.settings.interface.groups.status.blurb;
		},
		fields: colors([
			'--color-success',
			'--color-success-text',
			'--color-error',
			'--color-error-text',
			'--color-warning',
			'--color-warning-text',
			'--color-info',
			'--color-info-text'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.chatRoles.title;
		},
		get blurb() {
			return t.settings.interface.groups.chatRoles.blurb;
		},
		fields: colors([
			'--color-role-system',
			'--color-role-system-text',
			'--color-role-user',
			'--color-role-user-text',
			'--color-role-assistant',
			'--color-role-assistant-text'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.type.title;
		},
		get blurb() {
			return t.settings.interface.groups.type.blurb;
		},
		fields: [
			field('--font-family-heading', 'font'),
			field('--font-family-body', 'font'),
			...lengths([
				'--font-size-xs',
				'--font-size-sm',
				'--font-size-base',
				'--font-size-md',
				'--font-size-lg',
				'--font-size-xl',
				'--font-size-h1',
				'--font-size-h2',
				'--font-size-h3',
				'--font-size-h4'
			])
		]
	},
	{
		get title() {
			return t.settings.interface.groups.shape.title;
		},
		get blurb() {
			return t.settings.interface.groups.shape.blurb;
		},
		fields: lengths(['--radius-sm', '--radius-md', '--radius-lg', '--radius-xl', '--radius-2xl'])
	},
	{
		get title() {
			return t.settings.interface.groups.spacing.title;
		},
		get blurb() {
			return t.settings.interface.groups.spacing.blurb;
		},
		fields: lengths([
			'--spacing-xs',
			'--spacing-sm',
			'--spacing-md',
			'--spacing-lg',
			'--spacing-xl'
		])
	},
	{
		get title() {
			return t.settings.interface.groups.layout.title;
		},
		get blurb() {
			return t.settings.interface.groups.layout.blurb;
		},
		fields: [
			...lengths([
				'--width-sidebar-full',
				'--width-sidebar-small',
				'--width-sidebar-collapsed',
				'--width-content-max',
				'--height-header',
				'--height-input'
			]),
			field('--grid-card-min', 'text'),
			field('--avatar-aspect-ratio', 'text')
		]
	},
	{
		get title() {
			return t.settings.interface.groups.motion.title;
		},
		get blurb() {
			return t.settings.interface.groups.motion.blurb;
		},
		fields: [
			field('--transition-fast', 'text'),
			field('--transition-normal', 'text'),
			field('--transition-slow', 'text')
		]
	},
	{
		get title() {
			return t.settings.interface.groups.scrollbars.title;
		},
		get blurb() {
			return t.settings.interface.groups.scrollbars.blurb;
		},
		fields: [
			field('--scrollbar-width', 'px'),
			...colors(['--scrollbar-thumb', '--scrollbar-thumb-hover'])
		]
	}
];
