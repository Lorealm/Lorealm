import type { Style } from '$lib/stores/styles.svelte.js';

export const BUILT_IN_PREFIX = 'builtin:';

export function isBuiltIn(id: string): boolean {
	return id.startsWith(BUILT_IN_PREFIX);
}

export const CLASSIC_NAME = 'Lorealm Classic';

const githubDark: Style = {
	id: `${BUILT_IN_PREFIX}github-dark`,
	name: 'GitHub Dark',
	author: 'GitHub',
	css: '',
	tokens: {
		'--color-primary': '#2f81f7',
		'--color-primary-light': '#58a6ff',
		'--color-primary-lighter': '#79c0ff',
		'--color-primary-dark': '#1f6feb',
		'--color-primary-darker': '#1158c7',
		'--color-primary-dark-hover': 'rgba(47, 129, 247, 0.47)',

		'--color-bg-base': '#0d1117',
		'--color-bg-elevated': '#161b22',
		'--color-bg-elevated-2': '#1c2128',
		'--color-bg-elevated-3': '#21262d',
		'--color-bg-panel': 'rgba(13, 17, 23, 0.6)',
		'--color-bg-panel-solid': '#0d1117',
		'--color-bg-overlay': 'rgba(1, 4, 9, 0.4)',
		'--color-bg-input': '#010409',
		'--color-bg-modal': 'rgba(33, 38, 45, 0.98)',
		'--color-bg-disabled': '#21262d',

		'--color-text-primary': '#e6edf3',
		'--color-text-secondary': '#9198a1',
		'--color-text-muted': '#6e7681',
		'--color-text-disabled': '#484f58',
		'--color-text-accent': '#58a6ff',
		'--color-text-inverse': '#0d1117',

		'--color-border-subtle': '#21262d',
		'--color-border-medium': '#30363d',
		'--color-border-strong': '#3d444d',
		'--color-border-active': '#58a6ff',
		'--color-border-default': '#30363d',

		'--color-success': '#238636',
		'--color-success-bg': 'rgba(35, 134, 54, 0.2)',
		'--color-success-text': '#3fb950',
		'--color-success-border': 'rgba(63, 185, 80, 0.3)',

		'--color-error': '#da3633',
		'--color-error-bg': 'rgba(218, 54, 51, 0.2)',
		'--color-error-text': '#f85149',
		'--color-error-border': 'rgba(248, 81, 73, 0.3)',

		'--color-warning': '#9e6a03',
		'--color-warning-bg': 'rgba(187, 128, 9, 0.2)',
		'--color-warning-text': '#d29922',
		'--color-warning-border': 'rgba(210, 153, 34, 0.3)',

		'--color-info': '#1f6feb',
		'--color-info-bg': 'rgba(56, 139, 253, 0.2)',
		'--color-info-text': '#58a6ff',
		'--color-info-border': 'rgba(88, 166, 255, 0.3)',

		'--color-role-system': '#a371f7',
		'--color-role-system-bg': 'rgba(163, 113, 247, 0.15)',
		'--color-role-system-text': '#d2a8ff',

		'--color-role-user': '#388bfd',
		'--color-role-user-bg': 'rgba(56, 139, 253, 0.15)',
		'--color-role-user-text': '#79c0ff',

		'--color-role-assistant': '#3fb950',
		'--color-role-assistant-bg': 'rgba(63, 185, 80, 0.15)',
		'--color-role-assistant-text': '#56d364',

		'--color-hover': 'rgba(177, 186, 196, 0.12)',
		'--color-active': 'rgba(177, 186, 196, 0.2)',
		'--color-focus': 'rgba(56, 139, 253, 0.4)',
		'--color-selected': 'rgba(56, 139, 253, 0.15)',

		'--gradient-primary': 'linear-gradient(135deg, #2f81f7 0%, #1f6feb 100%)',
		'--gradient-bg': 'linear-gradient(135deg, #0d1117 0%, #161b22 100%)',
		'--gradient-portrait': 'linear-gradient(135deg, #388bfd 0%, #1f6feb 100%)',
		'--gradient-subtle':
			'linear-gradient(135deg, rgba(56, 139, 253, 0.1) 0%, rgba(56, 139, 253, 0.05) 100%)',

		'--shadow-sm': '0 1px 3px rgba(1, 4, 9, 0.5)',
		'--shadow-md': '0 3px 6px rgba(1, 4, 9, 0.6)',
		'--shadow-lg': '0 8px 24px rgba(1, 4, 9, 0.7)',
		'--shadow-xl': '0 12px 32px rgba(1, 4, 9, 0.8)',
		'--shadow-inset': 'inset 0 1px 3px rgba(1, 4, 9, 0.6)',
		'--shadow-glow': '0 0 0 3px rgba(56, 139, 253, 0.3)',

		'--color-overlay-bg': 'rgba(1, 4, 9, 0.7)',
		'--color-overlay-light': 'rgba(1, 4, 9, 0.4)',

		'--scrollbar-thumb': 'rgba(110, 118, 129, 0.4)',
		'--scrollbar-thumb-hover': 'rgba(110, 118, 129, 0.6)',

		'--font-family-heading': 'var(--font-family-system)',
		'--font-family-body': 'var(--font-family-system)',

		'--radius-sm': '4px',
		'--radius-md': '6px',
		'--radius-lg': '6px',
		'--radius-xl': '8px',
		'--radius-2xl': '12px'
	}
};

const catppuccinMocha: Style = {
	id: `${BUILT_IN_PREFIX}catppuccin-mocha`,
	name: 'Catppuccin Mocha',
	author: 'Catppuccin',
	css: '',
	tokens: {
		'--color-primary': '#cba6f7',
		'--color-primary-light': '#d7bbf9',
		'--color-primary-lighter': '#e3d0fb',
		'--color-primary-dark': '#b48ce8',
		'--color-primary-darker': '#9a72d0',
		'--color-primary-dark-hover': 'rgba(203, 166, 247, 0.47)',

		'--color-bg-base': '#181825',
		'--color-bg-elevated': '#1e1e2e',
		'--color-bg-elevated-2': '#252635',
		'--color-bg-elevated-3': '#313244',
		'--color-bg-panel': 'rgba(17, 17, 27, 0.6)',
		'--color-bg-panel-solid': '#11111b',
		'--color-bg-overlay': 'rgba(17, 17, 27, 0.4)',
		'--color-bg-input': '#11111b',
		'--color-bg-modal': 'rgba(49, 50, 68, 0.98)',
		'--color-bg-disabled': '#313244',

		'--color-text-primary': '#cdd6f4',
		'--color-text-secondary': '#a6adc8',
		'--color-text-muted': '#7f849c',
		'--color-text-disabled': '#585b70',
		'--color-text-accent': '#cba6f7',
		'--color-text-inverse': '#1e1e2e',

		'--color-border-subtle': '#252635',
		'--color-border-medium': '#313244',
		'--color-border-strong': '#45475a',
		'--color-border-active': '#cba6f7',
		'--color-border-default': '#313244',

		'--color-success': '#a6e3a1',
		'--color-success-bg': 'rgba(166, 227, 161, 0.15)',
		'--color-success-text': '#a6e3a1',
		'--color-success-border': 'rgba(166, 227, 161, 0.3)',

		'--color-error': '#f38ba8',
		'--color-error-bg': 'rgba(243, 139, 168, 0.15)',
		'--color-error-text': '#f38ba8',
		'--color-error-border': 'rgba(243, 139, 168, 0.3)',

		'--color-warning': '#f9e2af',
		'--color-warning-bg': 'rgba(249, 226, 175, 0.15)',
		'--color-warning-text': '#f9e2af',
		'--color-warning-border': 'rgba(249, 226, 175, 0.3)',

		'--color-info': '#89b4fa',
		'--color-info-bg': 'rgba(137, 180, 250, 0.15)',
		'--color-info-text': '#89b4fa',
		'--color-info-border': 'rgba(137, 180, 250, 0.3)',

		'--color-role-system': '#94e2d5',
		'--color-role-system-bg': 'rgba(148, 226, 213, 0.15)',
		'--color-role-system-text': '#94e2d5',

		'--color-role-user': '#f5c2e7',
		'--color-role-user-bg': 'rgba(245, 194, 231, 0.15)',
		'--color-role-user-text': '#f5c2e7',

		'--color-role-assistant': '#cba6f7',
		'--color-role-assistant-bg': 'rgba(203, 166, 247, 0.15)',
		'--color-role-assistant-text': '#cba6f7',

		'--color-hover': 'rgba(203, 166, 247, 0.12)',
		'--color-active': 'rgba(203, 166, 247, 0.2)',
		'--color-focus': 'rgba(203, 166, 247, 0.4)',
		'--color-selected': 'rgba(203, 166, 247, 0.15)',

		'--gradient-primary': 'linear-gradient(135deg, #cba6f7 0%, #b4befe 100%)',
		'--gradient-bg': 'linear-gradient(135deg, #1e1e2e 0%, #313244 100%)',
		'--gradient-portrait': 'linear-gradient(135deg, #f5c2e7 0%, #cba6f7 100%)',
		'--gradient-subtle':
			'linear-gradient(135deg, rgba(203, 166, 247, 0.1) 0%, rgba(203, 166, 247, 0.05) 100%)',

		'--shadow-sm': '0 2px 4px rgba(17, 17, 27, 0.4)',
		'--shadow-md': '0 4px 12px rgba(17, 17, 27, 0.5)',
		'--shadow-lg': '0 8px 24px rgba(17, 17, 27, 0.6)',
		'--shadow-xl': '0 12px 32px rgba(17, 17, 27, 0.7)',
		'--shadow-inset': 'inset 0 1px 3px rgba(17, 17, 27, 0.5)',
		'--shadow-glow': '0 0 16px rgba(203, 166, 247, 0.25)',

		'--color-overlay-bg': 'rgba(17, 17, 27, 0.7)',
		'--color-overlay-light': 'rgba(17, 17, 27, 0.4)',

		'--scrollbar-thumb': 'rgba(108, 112, 134, 0.5)',
		'--scrollbar-thumb-hover': 'rgba(127, 132, 156, 0.7)'
	}
};

export const BUILT_IN_STYLES: Style[] = [githubDark, catppuccinMocha];
