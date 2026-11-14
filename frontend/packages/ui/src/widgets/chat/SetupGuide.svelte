<script module lang="ts">
	import { connectionStore } from '#lib/stores/connection.svelte.js';
	import { tokenizerStore } from '#lib/stores/tokenizer.svelte.js';
	import { getProviderSchema } from '#lib/types/schema.js';
	import { templateStore } from '#lib/stores/template.svelte.js';
	import { personaStore } from '#lib/stores/persona.svelte.js';

	export function loadSetupState() {
		if (connectionStore.activeProfile === undefined)
			connectionStore.getActiveProfile().catch(console.error);
		if (connectionStore.profiles === undefined) connectionStore.loadProfiles().catch(console.error);
		tokenizerStore.ensureLoaded().catch(console.error);
	}

	function templateReady(): boolean {
		const template = templateStore.activeTemplate;
		return template != null;
	}

	function activePersona() {
		const id = personaStore.activePersonaId;
		return id == null ? undefined : personaStore.loadedPersonas?.find((p) => p.id === id);
	}

	function personaReady(): boolean {
		return activePersona() != undefined;
	}

	function activeProfile() {
		const id = connectionStore.activeProfile;
		return id == null ? undefined : connectionStore.profiles?.find((p) => p.id === id);
	}

	function connectionReady(): boolean {
		const profile = activeProfile();
		if (!profile) return false;
		return (
			profile.secret_id != null || !(getProviderSchema(profile.provider)?.requiresKey ?? false)
		);
	}

	export function setupKnown(): boolean {
		return (
			connectionStore.activeProfile !== undefined &&
			connectionStore.profiles !== undefined &&
			tokenizerStore.loaded
		);
	}

	export function setupComplete(): boolean {
		return connectionReady() && tokenizerStore.activeId != null;
	}
</script>

<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { Check, ChevronRight } from '@lucide/svelte';
	import { navigator } from '#lib/navigation/navigator.js';

	const openTokenizers = () => navigator.start().settings().user().tokenizers();

	const connectionHint = $derived.by(() => {
		const profile = activeProfile();
		if (!profile) return t.chat.setup.noProfile;
		if (connectionReady())
			return t.chat.setup.usingProfile(profile.name || t.chat.setup.yourProfileLower);
		const label = getProviderSchema(profile.provider)?.label ?? profile.provider;
		return t.chat.setup.needsKey(profile.name || t.chat.setup.yourProfile, label);
	});

	const steps = $derived([
		{
			done: templateReady(),
			title: t.chat.setup.template.title,
			hint: t.chat.setup.template.hint,
			go: () => navigator.start().studio().templates()
		},
		{
			done: personaReady(),
			title: t.chat.setup.persona.title,
			hint: t.chat.setup.persona.hint,
			go: () => navigator.start().studio().personas()
		},
		{
			done: connectionReady(),
			title: t.chat.setup.connection.title,
			hint: connectionHint,
			// Open the active profile directly; its editor is where the key gets attached.
			go: () => navigator.start().connections(activeProfile()?.id)
		},
		{
			done: tokenizerStore.tokenizers.length > 0,
			title: t.chat.setup.installTokenizer.title,
			hint: t.chat.setup.installTokenizer.hint,
			go: openTokenizers
		},
		{
			done: tokenizerStore.activeId != null,
			title: t.chat.setup.activeTokenizer.title,
			hint: t.chat.setup.activeTokenizer.hint,
			go: openTokenizers
		}
	]);
</script>

<section class="guide" aria-labelledby="setup-guide-title">
	<h2 id="setup-guide-title">{t.chat.setup.title}</h2>
	<p class="lead">{t.chat.setup.lead}</p>

	<ol>
		{#each steps as step (step.title)}
			<li>
				<button type="button" class="step" class:done={step.done} onclick={step.go}>
					<span class="mark" aria-hidden="true">
						{#if step.done}<Check size={14} strokeWidth={3} />{/if}
					</span>
					<span class="text">
						<span class="title">{step.title}</span>
						<span class="hint">{step.hint}</span>
					</span>
					<span class="sr-only">{step.done ? t.chat.setup.done : t.chat.setup.todo}</span>
					<ChevronRight size={16} class="chevron" aria-hidden="true" />
				</button>
			</li>
		{/each}
	</ol>
</section>

<style>
	.guide {
		margin: auto;
		width: min(440px, 100%);
		box-sizing: border-box;
		padding: var(--spacing-xl);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-xl);
	}
	h2 {
		margin: 0 0 var(--spacing-xs);
		font-size: var(--font-size-h5);
		color: var(--color-text-primary);
	}
	.lead {
		margin: 0 0 var(--spacing-lg);
		color: var(--color-text-secondary);
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.step {
		width: 100%;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-sm) var(--spacing-md);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.step:hover {
		background: var(--color-hover);
	}
	.mark {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		border-radius: var(--radius-sm);
		border: 1.5px solid var(--color-border-active);
		color: var(--color-text-inverse);
	}
	.done .mark {
		background: var(--color-success);
		border-color: var(--color-success);
	}
	.text {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.title {
		color: var(--color-text-primary);
	}
	.hint {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.done .title {
		color: var(--color-text-muted);
		text-decoration: line-through;
	}
	.step :global(.chevron) {
		flex: 0 0 auto;
		color: var(--color-text-muted);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
