import { EditorView } from '@codemirror/view';

// Shared chrome for every weaver editor/field: colors, fonts, gutter, cursor,
// selection, tooltip. Deliberately NO sizing (height/padding) — that's
// layout-specific and lives in each component. Token colors are separate
// again, in weaver-lang.ts (HighlightStyle).
//
// Colors/fonts now resolve from variables.css (the shared design tokens),
// so this editor stays in sync with the rest of the UI.
export const weaverTheme = EditorView.theme(
	{
		'&': {
			color: 'var(--color-text-primary)',
			backgroundColor: 'var(--color-bg-base)',
			fontSize: 'var(--font-size-md)'
		},
		'.cm-content': {
			fontFamily: 'var(--font-family-code)',
			caretColor: 'var(--color-primary-light)'
		},
		'.cm-gutters': {
			backgroundColor: 'var(--color-bg-base)',
			color: 'var(--color-text-muted)',
			border: 'none',
			borderRight: '1px solid var(--color-border-subtle)'
		},
		'.cm-activeLineGutter': {
			backgroundColor: 'var(--color-bg-elevated-2)',
			color: 'var(--color-text-secondary)'
		},
		'.cm-activeLine': { backgroundColor: 'var(--color-bg-elevated)' },
		'.cm-cursor': { borderLeftColor: 'var(--color-primary-light)' },
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
			backgroundColor: 'var(--color-active)'
		},
		'.cm-tooltip': { border: 'none', backgroundColor: 'transparent' }
	},
	{ dark: true }
);
