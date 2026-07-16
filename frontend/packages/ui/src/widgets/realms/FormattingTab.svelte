<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { REALM_CONFIG_DEFAULTS, type RealmConfig } from '$lib/types/realm.js';
	import { RotateCcw } from '@lucide/svelte';

	type Field = keyof RealmConfig;

	const ROWS: Array<{ field: Field; label: string; help: string; vars: string }> = [
		{
			field: 'speech_format',
			label: 'Speech',
			help: "How another agent's speech reads in an observer's feed.",
			vars: '{name} {content}'
		},
		{
			field: 'action_format',
			label: 'Action',
			help: "An acted event's content — thin by design, no result details.",
			vars: '{name} {tool}'
		},
		{
			field: 'tool_result_format',
			label: 'Tool Result',
			help: "The actor's private note after a tool runs.",
			vars: '{tool} {output}'
		},
		{
			field: 'state_change_format',
			label: 'State Change',
			help: "A state_changed event's content.",
			vars: '{scope} {changes}'
		},
		{
			field: 'state_write_format',
			label: 'State Write',
			help: 'One written variable within a state change.',
			vars: '{key} {value}'
		},
		{
			field: 'state_delete_format',
			label: 'State Delete',
			help: 'One deleted variable within a state change.',
			vars: '{key}'
		}
	];

	const SAMPLE: Record<string, string> = {
		name: 'Aria',
		content: '"Uhm... Hello?..."',
		tool: 'Forge Documents',
		output: 'papers accepted',
		scope: 'nation',
		changes: 'velen.treasury: 812 → 640',
		key: 'velen.treasury',
		value: '640'
	};

	const fmt = (tpl: string) =>
		tpl.replace(/\{(\w+)\}/g, (_, k) => (SAMPLE[k] !== undefined ? SAMPLE[k] : `{${k}}`));

	let draft = $state<Record<string, string>>(toDraft(realmStore.config));

	function toDraft(config: RealmConfig): Record<string, string> {
		const d: Record<string, string> = {};
		for (const { field } of ROWS) d[field] = config[field] ?? '';
		return d;
	}

	function persist() {
		const config: RealmConfig = {};
		for (const { field } of ROWS) config[field] = draft[field].trim() === '' ? null : draft[field];
		void realmStore.saveConfig(config);
	}

	let timer: ReturnType<typeof setTimeout> | undefined;
	function saveSoon() {
		clearTimeout(timer);
		timer = setTimeout(persist, 500);
	}

	function reset(field: Field) {
		draft[field] = '';
		persist();
	}
</script>

<div class="stage">
	<div class="lede">
		How events read once they reach an observation feed. Blank fields use the default formatting.
	</div>
	{#each ROWS as row (row.field)}
		{@const overridden = draft[row.field].trim() !== ''}
		{@const effective = overridden ? draft[row.field] : REALM_CONFIG_DEFAULTS[row.field]}
		<div class="card">
			<div class="rowBody">
				<div class="rowHead">
					<div class="label">{row.label}</div>
					<div class="vars">{row.vars}</div>
				</div>
				<div class="help">{row.help}</div>
				<input
					type="text"
					class="mono"
					bind:value={draft[row.field]}
					placeholder={REALM_CONFIG_DEFAULTS[row.field]}
					oninput={saveSoon}
				/>
				<div class="preview">Preview: <span class="mono">{fmt(effective)}</span></div>
			</div>
			{#if overridden}
				<button
					type="button"
					class="icon"
					title="Reset to default"
					onclick={() => reset(row.field)}
				>
					<RotateCcw size={14} />
				</button>
			{/if}
		</div>
	{/each}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		max-width: var(--width-content-max);
	}

	.lede {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
		line-height: var(--line-height-normal);
	}

	.card {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-lg);
		padding: var(--spacing-md);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
	}

	.rowBody {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.rowHead {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.label {
		font-family: var(--font-family-heading);
		font-weight: 600;
	}

	.vars {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-family: var(--font-family-mono);
	}

	.help {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.mono {
		font-family: var(--font-family-mono);
	}

	.preview {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}

	.preview .mono {
		color: var(--color-text-primary);
	}

	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background: none;
		border: none;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.icon:hover {
		color: var(--color-text-primary);
	}
</style>
