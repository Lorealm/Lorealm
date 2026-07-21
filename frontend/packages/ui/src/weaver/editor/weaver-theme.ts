import { EditorView } from '@codemirror/view';

export const weaverTheme = EditorView.theme(
	{
		'&': {
			color: 'var(--color-text-primary)',
			backgroundColor: 'var(--color-bg-base)',
			fontSize: 'var(--font-size-base)'
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
		'.cm-activeLine': {
			backgroundColor: 'color-mix(in srgb, var(--color-bg-elevated) 10%, transparent)'
		},
		'.cm-cursor': { borderLeftColor: 'var(--color-primary-light)' },
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
			backgroundColor: 'var(--color-active)'
		},
		'.cm-tooltip': { border: 'none', backgroundColor: 'transparent' },
		'.cm-tooltip.cm-tooltip-autocomplete': {
			backgroundColor: 'var(--color-bg-elevated-2)',
			border: '1px solid var(--color-border-subtle)',
			borderRadius: '6px',
			boxShadow: '0 6px 20px rgba(0, 0, 0, 0.45)'
		},
		'.cm-tooltip-autocomplete > ul': {
			fontFamily: 'var(--font-family-code)',
			fontSize: 'var(--font-size-base)',
			maxHeight: '16em'
		},
		'.cm-tooltip-autocomplete > ul > li': {
			padding: '3px 8px',
			color: 'var(--color-text-primary)'
		},
		'.cm-tooltip-autocomplete > ul > li[aria-selected]': {
			backgroundColor: 'var(--color-primary)',
			color: 'var(--color-text-inverse)'
		},
		'.cm-completionDetail': {
			marginLeft: '0.75em',
			color: 'var(--color-text-muted)',
			fontStyle: 'normal'
		},
		'.cm-completionMatchedText': {
			textDecoration: 'none',
			fontWeight: '600',
			color: 'var(--color-text-accent)'
		},
		'.cm-tooltip-autocomplete > ul > li[aria-selected] .cm-completionMatchedText': {
			color: 'var(--color-text-inverse)'
		},
		'.cm-tooltip-autocomplete > ul > li[aria-selected] .cm-completionDetail': {
			color: 'var(--color-text-inverse)',
			opacity: '0.7'
		},
		'&.cm-weaver-mod .cm-weaver-ref': {
			cursor: 'pointer',
			textDecoration: 'underline',
			textDecorationThickness: '2px',
			color: 'var(--color-text-accent)'
		},
		'.cm-weaver-tooltip': {
			backgroundColor: 'var(--color-bg-elevated-2)',
			border: '1px solid var(--color-border-subtle)',
			borderRadius: '6px',
			padding: '8px 10px',
			maxWidth: '360px',
			boxShadow: '0 6px 20px rgba(0, 0, 0, 0.45)',
			color: 'var(--color-text-primary)',
			fontFamily: 'var(--font-family-code)',
			whiteSpace: 'normal'
		},
		'.cm-weaver-tooltip-sig': { fontWeight: '600', marginBottom: '4px' },
		'.cm-weaver-tooltip-doc': { opacity: '0.85' },
		'.cm-weaver-tooltip-params': {
			margin: '6px 0 0',
			paddingTop: '6px',
			borderTop: '1px solid var(--color-border-subtle)'
		},
		'.cm-weaver-tooltip-params dt': {
			color: 'var(--color-text-accent)',
			fontWeight: '600'
		},
		'.cm-weaver-tooltip-params dd': {
			margin: '0 0 4px',
			opacity: '0.85'
		},
		'.cm-weaver-tooltip-params dd:last-child': { marginBottom: '0' },
		'.cm-weaver-tooltip-doc code': {
			backgroundColor: 'var(--color-bg-base)',
			padding: '0 3px',
			borderRadius: '3px'
		}
	},
	{ dark: true }
);
