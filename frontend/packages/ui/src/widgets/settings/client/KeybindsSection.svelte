<script lang="ts">
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { Plus, RotateCcw, X } from '@lucide/svelte';

	import KeyChord from '$lib/components/KeyChord.svelte';
	import { chordsFromEvent, type Chord } from '$lib/keys/chord.js';
	import { keybindConfig, type KeybindMode } from '$lib/keys/config.svelte.js';
	import { inputEnvironment } from '$lib/keys/environment.svelte.js';
	import { keybinds } from '$lib/keys/manager.svelte.js';
	import { commandRegistry } from '$lib/keys/registry.svelte.js';
	import type { RegisteredCommand } from '$lib/keys/types.js';
	import { VIEW_LABEL } from '$lib/navigation/breadcrumbs.svelte.js';
	import type { ViewName } from '$lib/navigation/routes.js';

	const MODES: Array<{ value: KeybindMode; label: string }> = [
		{ value: 'auto', label: 'Automatic' },
		{ value: 'on', label: 'Always on' },
		{ value: 'off', label: 'Off' }
	];

	let capturing = $state<{ id: string; append: boolean } | null>(null);

	const commands = $derived(commandRegistry.all.filter((command) => !command.hidden));

	const groups = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const byName = new Map<string, RegisteredCommand[]>();
		for (const command of commands) {
			const group = byName.get(command.group);
			if (group) group.push(command);
			else byName.set(command.group, [command]);
		}
		return [...byName];
	});

	function scopeLabel(command: RegisteredCommand): string {
		switch (command.scope) {
			case 'global':
				return 'Global';
			case 'view':
				return `${VIEW_LABEL[command.layer as ViewName] ?? command.layer} view`;
			default:
				return command.layer;
		}
	}

	function startCapture(command: RegisteredCommand, append: boolean) {
		capturing = { id: command.id, append };
		keybinds.suspended = true;
	}

	function stopCapture() {
		capturing = null;
		keybinds.suspended = false;
	}

	function onCapture(event: KeyboardEvent) {
		const active = capturing;
		if (!active) return;

		event.preventDefault();
		event.stopPropagation();

		if (event.key === 'Escape') {
			stopCapture();
			return;
		}

		const [chord] = chordsFromEvent(event);
		if (!chord) return;

		const command = commandRegistry.get(active.id);
		if (!command) {
			stopCapture();
			return;
		}

		const existing = active.append ? commandRegistry.keysFor(command) : [];
		if (!existing.includes(chord)) {
			void keybindConfig.setKeys(command.id, [...existing, chord]);
		}
		stopCapture();
	}

	function unbind(command: RegisteredCommand, chord: Chord) {
		const remaining = commandRegistry.keysFor(command).filter((other) => other !== chord);
		void keybindConfig.setKeys(command.id, remaining);
	}
</script>

<svelte:window onkeydowncapture={onCapture} />

<section class="section">
	<div class="body">
		<h2>Keybinds</h2>

		<div class="mode">
			<label class="field">
				<span>Keyboard shortcuts</span>
				<select
					value={keybindConfig.mode}
					onchange={(e) => keybindConfig.setMode(e.currentTarget.value as KeybindMode)}
				>
					{#each MODES as mode (mode.value)}
						<option value={mode.value}>{mode.label}</option>
					{/each}
				</select>
			</label>
			<span class="hint">
				{#if keybindConfig.mode === 'auto'}
					Off automatically on phones and tablets, where there is no keyboard to press.
				{:else if keybindConfig.mode === 'on'}
					On everywhere, including touch devices with a keyboard attached.
				{:else}
					Every shortcut below is disabled.
				{/if}
			</span>
			{#if !keybinds.enabled}
				<p class="notice">
					Shortcuts are inactive on this device{inputEnvironment.touchOnly
						? ' — it has no hover-capable pointer'
						: ''}. Choose <strong>Always on</strong> to use them anyway.
				</p>
			{/if}
		</div>

		{#if capturing}
			<p class="notice recording">Press the keys to bind. Escape cancels.</p>
		{/if}

		{#each groups as [group, entries] (group)}
			<div class="group">
				<h3>{group}</h3>
				{#each entries as command (command.id)}
					{@const chords = commandRegistry.keysFor(command)}
					<div class="row">
						<div class="what">
							<span class="title">
								{command.title}
								<span class="scope">{scopeLabel(command)}</span>
							</span>
							{#if command.description}
								<span class="hint">{command.description}</span>
							{/if}
							{#each chords as chord (chord)}
								{@const clashes = commandRegistry.conflictsWith(command, chord)}
								{#if clashes.length > 0}
									<span class="clash">
										Also bound to {clashes.map((other) => other.title).join(', ')}.
									</span>
								{/if}
							{/each}
						</div>

						<div class="keys">
							{#each chords as chord (chord)}
								<span class="chip">
									<KeyChord {chord} />
									<button
										class="icon"
										{@attach tooltip('Unbind')}
										aria-label="Unbind {chord}"
										onclick={() => unbind(command, chord)}
									>
										<X size={12} />
									</button>
								</span>
							{:else}
								<span class="unbound">Unbound</span>
							{/each}

							<button
								class="action"
								class:active={capturing?.id === command.id}
								onclick={() => startCapture(command, chords.length > 0)}
							>
								{#if capturing?.id === command.id}
									Press a key…
								{:else if chords.length > 0}
									<Plus size={12} /> Add
								{:else}
									Record
								{/if}
							</button>

							{#if keybindConfig.isOverridden(command.id)}
								<button
									class="icon"
									{@attach tooltip('Reset to default')}
									aria-label="Reset {command.title} to default"
									onclick={() => keybindConfig.reset(command.id)}
								>
									<RotateCcw size={12} />
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/each}

		<div class="footer">
			<button onclick={() => keybindConfig.resetAll()}>Reset all to defaults</button>
		</div>
	</div>
</section>

<style>
	.section {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		flex-direction: column;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
		max-width: var(--width-content-max);
	}
	.body h2 {
		font-size: var(--font-size-h4);
	}
	.mode {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.field {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	select {
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-input);
		color: var(--color-text-primary);
		font-family: var(--font-family-body);
		font-size: var(--font-size-base);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.notice {
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-info-border);
		border-radius: var(--radius-md);
		background-color: var(--color-info-bg);
		color: var(--color-info-text);
		font-size: var(--font-size-sm);
	}
	.notice.recording {
		position: sticky;
		top: 0;
		z-index: var(--z-index-sticky);
		border-color: var(--color-border-active);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-accent);
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-lg);
		padding: var(--spacing-sm);
		border-radius: var(--radius-md);
	}
	.row:hover {
		background-color: var(--color-bg-elevated-2);
	}
	.what {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.title {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.scope {
		padding: 1px var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		white-space: nowrap;
	}
	.clash {
		color: var(--color-warning-text);
		font-size: var(--font-size-xs);
	}
	.keys {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--spacing-xs);
		flex-shrink: 0;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 2px;
	}
	.unbound {
		color: var(--color-text-disabled);
		font-size: var(--font-size-xs);
	}
	button.icon {
		padding: var(--spacing-xs);
		background: none;
		color: var(--color-text-muted);
	}
	button.icon:hover {
		color: var(--color-error-text);
	}
	button.action {
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-border-medium);
		font-size: var(--font-size-xs);
	}
	button.action.active {
		border-color: var(--color-border-active);
		color: var(--color-text-accent);
	}
	.footer {
		display: flex;
		padding-top: var(--spacing-sm);
		border-top: 1px solid var(--color-border-subtle);
	}
</style>
