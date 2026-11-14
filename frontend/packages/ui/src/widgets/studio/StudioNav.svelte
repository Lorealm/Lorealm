<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { StudioSection } from '#lib/navigation/routes.js';
	import { importFiles } from '#lib/stores/transfer.js';
	import { Upload } from '@lucide/svelte';

	let { section }: { section: StudioSection } = $props();

	let inputEl: HTMLInputElement;

	function onPick(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = Array.from(input.files ?? []);
		input.value = '';
		if (files.length > 0) void importFiles(files);
	}

	const sections: Array<{ id: StudioSection; go: () => void }> = [
		{ id: 'characters', go: () => navigator.start().studio().characters() },
		{ id: 'lorebooks', go: () => navigator.start().studio().lorebooks() },
		{ id: 'templates', go: () => navigator.start().studio().templates() },
		{ id: 'tools', go: () => navigator.start().studio().tools() },
		{ id: 'modules', go: () => navigator.start().studio().modules() },
		{ id: 'personas', go: () => navigator.start().studio().personas() },
		{ id: 'widgets', go: () => navigator.start().studio().widgets() }
	];
</script>

<nav class="studioNav">
	{#each sections as { id, go } (id)}
		<button class="tab" class:active={section === id} onclick={go}>
			{t.nav.studio[id]}
		</button>
	{/each}
	<input
		bind:this={inputEl}
		type="file"
		multiple
		accept="image/png,.png,application/json,.json"
		onchange={onPick}
		hidden
		style="display:none"
	/>
	<button class="import" onclick={() => inputEl.click()}>
		<Upload size={16} />
		<span class="import-label">{t.studio.nav.import}</span>
	</button>
</nav>

<style>
	.studioNav {
		display: flex;
		gap: var(--spacing-xs);
		padding: var(--spacing-md);
		background-color: var(--color-bg-panel);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.tab {
		display: flex;
		justify-content: center;
		padding: var(--spacing-xs) var(--spacing-md);
		border-radius: var(--radius-full);
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
		align-items: center;
		height: auto;
		flex: 0 1;
	}
	.import {
		justify-content: center;
		padding: var(--spacing-sm) var(--spacing-md);
		margin-left: auto;
		background-color: var(--color-bg-elevated-3);
	}
	.tab.active {
		border-color: currentColor;
	}
	.import-label {
		margin-left: var(--spacing-sm);
	}

	@media (max-width: 768px) {
		.studioNav {
			padding: var(--spacing-sm);
			overflow-x: auto;
			scrollbar-width: none;
		}
		.studioNav > * {
			flex-shrink: 0;
			white-space: nowrap;
		}
	}
</style>
