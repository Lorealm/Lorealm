<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { inspectorFocus } from '#lib/stores/inspectorFocus.svelte.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import { ChevronDown } from '@lucide/svelte';
	import DOMPurify from 'dompurify';
	import { Marked } from 'marked';
	import type { Attachment } from 'svelte/attachments';

	const markdown = new Marked({ breaks: true });

	const BASE_STYLE = `<style>
		:host { display: block; overflow-wrap: anywhere; }
		img, video { max-width: 100%; height: auto; }
		a { color: var(--color-text-accent); }
		pre { white-space: pre-wrap; }
	</style>`;

	let collapsed = $state(false);

	const overriding = $derived(
		router.current.view === 'characters' && inspectorFocus.characterId !== null
	);
	const character = $derived(
		overriding ? inspectorFocus.character : characterStore.activeCharacter
	);
	const loading = $derived(overriding && inspectorFocus.character === null);
	const notes = $derived(character?.metadata.creator_notes.trim() ?? '');
	const html = $derived(
		notes
			? DOMPurify.sanitize(markdown.parse(notes, { async: false }), {
					FORCE_BODY: true,
					FORBID_TAGS: ['form', 'input', 'button', 'textarea', 'select', 'option']
				})
			: ''
	);

	function render(content: string): Attachment<HTMLElement> {
		return (host) => {
			const root = host.shadowRoot ?? host.attachShadow({ mode: 'open' });
			// SAFETY: Sanitized with DOMPurify above.
			root.innerHTML = BASE_STYLE + content;

			const onclick = (event: Event) => {
				const link = (event.target as Element | null)?.closest('a');
				const href = link?.getAttribute('href');
				if (!link || !href) return;
				event.preventDefault();
				if (href.startsWith('#')) {
					let id = href.slice(1);
					try {
						id = decodeURIComponent(id);
					} catch {
						// Malformed escape; look it up as written.
					}
					root.getElementById(id)?.scrollIntoView();
				} else {
					window.open(link.href, '_blank', 'noopener');
				}
			};
			root.addEventListener('click', onclick);
			return () => root.removeEventListener('click', onclick);
		};
	}
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">{t.inspector.widgets.creatorNotes}</span>
		{#if character}
			<span class="who">{character.metadata.display_name || character.definition.name}</span>
		{/if}
	</button>

	{#if !collapsed}
		{#if loading}
			<div class="empty">{t.inspector.creatorNotes.loading}</div>
		{:else if !character}
			<div class="empty">{t.inspector.creatorNotes.noCharacter}</div>
		{:else if !html}
			<div class="empty">{t.inspector.creatorNotes.empty}</div>
		{:else}
			<div class="body">
				<div class="notes" {@attach render(html)}></div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-huge, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		text-align: left;
		cursor: pointer;
	}
	.label {
		flex: 0 0 auto;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.who {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: right;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev :global(svg) {
		width: 14px;
		height: 14px;
	}

	.body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-sm) var(--spacing-md);
	}
	.notes {
		position: relative;
		contain: layout paint;
		isolation: isolate;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-normal);
	}
	.empty {
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
