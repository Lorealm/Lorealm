/**
 * The tokens the Appearance editor puts a labelled control behind.
 *
 * Hand-picked rather than read off `variables.css`: the stylesheet holds plenty
 * a reader has no business touching one at a time (the alpha-derived status
 * backgrounds, the shadow stacks), and nothing there says what a token is *for*.
 * Anything left out is still reachable through the custom CSS block, so this
 * list is about what to put in front of someone, not what they are allowed.
 */

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
	{ label: 'System', value: 'var(--font-family-system)' },
	{ label: 'Monospace', value: 'var(--font-family-mono)' }
];

const colors = (entries: Array<[string, string]>): TokenField[] =>
	entries.map(([name, label]) => ({ name, label, kind: 'color' }));

const lengths = (entries: Array<[string, string]>): TokenField[] =>
	entries.map(([name, label]) => ({ name, label, kind: 'px' }));

export const APPEARANCE_GROUPS: TokenGroup[] = [
	{
		title: 'Accent',
		blurb: 'The colour the client is built around. Most borders and highlights derive from it.',
		fields: colors([
			['--color-primary', 'Accent'],
			['--color-primary-light', 'Accent, light'],
			['--color-primary-lighter', 'Accent, lighter'],
			['--color-primary-dark', 'Accent, dark'],
			['--color-primary-darker', 'Accent, darker']
		])
	},
	{
		title: 'Surfaces',
		blurb: 'Backgrounds; window colors, modals and dialogs',
		fields: colors([
			['--color-bg-base', 'Window'],
			['--color-bg-elevated', 'Raised'],
			['--color-bg-elevated-2', 'Raised, higher'],
			['--color-bg-elevated-3', 'Raised, highest'],
			['--color-bg-panel-solid', 'Panel'],
			['--color-bg-input', 'Input field'],
			['--color-bg-modal', 'Dialog'],
			['--color-bg-disabled', 'Disabled']
		])
	},
	{
		title: 'Text',
		blurb: 'Foreground colours, in descending prominence.',
		fields: colors([
			['--color-text-primary', 'Body'],
			['--color-text-secondary', 'Secondary'],
			['--color-text-muted', 'Muted'],
			['--color-text-disabled', 'Disabled'],
			['--color-text-accent', 'Accented'],
			['--color-text-inverse', 'On accent']
		])
	},
	{
		title: 'Borders',
		blurb: 'Dividers and outlines.',
		fields: colors([
			['--color-border-subtle', 'Subtle'],
			['--color-border-medium', 'Medium'],
			['--color-border-strong', 'Strong'],
			['--color-border-active', 'Active'],
			['--color-border-default', 'Neutral']
		])
	},
	{
		title: 'Interaction',
		blurb: 'Conditional colors for UI feedback',
		fields: colors([
			['--color-hover', 'Hover'],
			['--color-active', 'Pressed'],
			['--color-focus', 'Focus ring'],
			['--color-selected', 'Selected']
		])
	},
	{
		title: 'Status',
		blurb: 'Toasts and inline feedback. The matching backgrounds are derived from these.',
		fields: colors([
			['--color-success', 'Success'],
			['--color-success-text', 'Success text'],
			['--color-error', 'Error'],
			['--color-error-text', 'Error text'],
			['--color-warning', 'Warning'],
			['--color-warning-text', 'Warning text'],
			['--color-info', 'Info'],
			['--color-info-text', 'Info text']
		])
	},
	{
		title: 'Chat roles',
		blurb: 'How the three speakers are told apart in a conversation.',
		fields: colors([
			['--color-role-system', 'System'],
			['--color-role-system-text', 'System text'],
			['--color-role-user', 'You'],
			['--color-role-user-text', 'Your text'],
			['--color-role-assistant', 'Assistant'],
			['--color-role-assistant-text', 'Assistant text']
		])
	},
	{
		title: 'Type',
		blurb: 'Families and the size scale. Every size in the client is one of these.',
		fields: [
			{ name: '--font-family-heading', label: 'Headings', kind: 'font' },
			{ name: '--font-family-body', label: 'Body', kind: 'font' },
			...lengths([
				['--font-size-xs', 'Extra small'],
				['--font-size-sm', 'Small'],
				['--font-size-base', 'Base'],
				['--font-size-md', 'Medium'],
				['--font-size-lg', 'Large'],
				['--font-size-xl', 'Extra large'],
				['--font-size-h1', 'Heading 1'],
				['--font-size-h2', 'Heading 2'],
				['--font-size-h3', 'Heading 3'],
				['--font-size-h4', 'Heading 4']
			])
		]
	},
	{
		title: 'Shape',
		blurb: 'Corner rounding.',
		fields: lengths([
			['--radius-sm', 'Small'],
			['--radius-md', 'Medium'],
			['--radius-lg', 'Large'],
			['--radius-xl', 'Extra large'],
			['--radius-2xl', 'Largest']
		])
	},
	{
		title: 'Spacing',
		blurb: 'The gap scale. Affects the entire application.',
		fields: lengths([
			['--spacing-xs', 'Extra small'],
			['--spacing-sm', 'Small'],
			['--spacing-md', 'Medium'],
			['--spacing-lg', 'Large'],
			['--spacing-xl', 'Extra large']
		])
	},
	{
		title: 'Layout',
		blurb: 'Fixed dimensions of the frame around your content.',
		fields: [
			...lengths([
				['--width-sidebar-full', 'Sidebar'],
				['--width-sidebar-small', 'Inspector'],
				['--width-sidebar-collapsed', 'Sidebar, collapsed'],
				['--width-content-max', 'Content width'],
				['--height-header', 'Header height'],
				['--height-input', 'Control height']
			]),
			{ name: '--grid-card-min', label: 'Card width', kind: 'text' },
			{ name: '--avatar-aspect-ratio', label: 'Avatar ratio', kind: 'text' }
		]
	},
	{
		title: 'Motion',
		blurb: 'Transition durations. Set them to 0s to turn animation off.',
		fields: [
			{ name: '--transition-fast', label: 'Fast', kind: 'text' },
			{ name: '--transition-normal', label: 'Normal', kind: 'text' },
			{ name: '--transition-slow', label: 'Slow', kind: 'text' }
		]
	},
	{
		title: 'Scrollbars',
		blurb: 'The thin scrollbars the client draws over the native ones.',
		fields: [
			{ name: '--scrollbar-width', label: 'Width', kind: 'px' },
			...colors([
				['--scrollbar-thumb', 'Thumb'],
				['--scrollbar-thumb-hover', 'Thumb, hovered']
			])
		]
	}
];
