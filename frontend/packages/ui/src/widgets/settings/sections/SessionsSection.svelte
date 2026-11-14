<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import type { SlidingPolicy, CsrfRotationPolicy } from '#lib/types/config.js';
	import SettingsSection from '../SettingsSection.svelte';
	import Field from '../Field.svelte';
	import NumberField from '#lib/components/NumberInput.svelte';

	let draft = $state($state.snapshot(configStore.config!.sessions));
	let saveState = $state<SaveState>({ status: 'idle' });

	const dirty = $derived(JSON.stringify(draft) !== JSON.stringify(configStore.config!.sessions));

	const slidingTag = $derived(
		typeof draft.sliding_policy === 'string' ? draft.sliding_policy : 'Window'
	);
	function setSlidingTag(tag: string) {
		draft.sliding_policy =
			tag === 'Window' ? ({ Window: 3600 } as SlidingPolicy) : (tag as SlidingPolicy);
	}

	const csrfTag = $derived(
		typeof draft.csrf_rotation_policy === 'string'
			? draft.csrf_rotation_policy
			: 'Periodic' in draft.csrf_rotation_policy
				? 'Periodic'
				: 'AfterRequests'
	);
	function setCsrfTag(tag: string) {
		if (tag === 'Periodic') draft.csrf_rotation_policy = { Periodic: 3600 };
		else if (tag === 'AfterRequests') draft.csrf_rotation_policy = { AfterRequests: 100 };
		else draft.csrf_rotation_policy = tag as CsrfRotationPolicy;
	}

	function reset() {
		draft = $state.snapshot(configStore.config!.sessions);
		saveState = { status: 'idle' };
	}

	async function save() {
		saveState = { status: 'saving' };
		try {
			await configStore.patch('sessions', $state.snapshot(draft));
			draft = $state.snapshot(configStore.config!.sessions);
			saveState = { status: 'saved' };
		} catch (e) {
			saveState = { status: 'error', message: e instanceof Error ? e.message : String(e) };
		}
	}
</script>

<SettingsSection title={t.nav.server.sessions} {dirty} {saveState} onsave={save} onreset={reset}>
	<Field label={t.settings.server.tokenLength}>
		<NumberField
			value={draft.token_length}
			min={1}
			onchange={(v) => v !== '' && (draft.token_length = v)}
		/>
	</Field>
	<Field label={t.settings.server.csrfTokenLength}>
		<NumberField
			value={draft.csrf_token_length}
			min={1}
			onchange={(v) => v !== '' && (draft.csrf_token_length = v)}
		/>
	</Field>
	<Field label={t.settings.server.ttl}>
		<NumberField
			value={draft.ttl_seconds}
			min={0}
			onchange={(v) => v !== '' && (draft.ttl_seconds = v)}
		/>
	</Field>

	<Field label={t.settings.server.slidingPolicy} hint={t.settings.server.slidingHint}>
		<select value={slidingTag} onchange={(e) => setSlidingTag(e.currentTarget.value)}>
			<option value="Fixed">{t.settings.server.fixed}</option>
			<option value="Always">{t.settings.server.always}</option>
			<option value="Window">{t.settings.server.window}</option>
		</select>
	</Field>
	{#if typeof draft.sliding_policy !== 'string'}
		<Field label={t.settings.server.windowSeconds}>
			<NumberField
				value={draft.sliding_policy.Window}
				min={0}
				onchange={(v) => v !== '' && (draft.sliding_policy = { Window: v })}
			/>
		</Field>
	{/if}

	<Field label={t.settings.server.csrfRotation}>
		<select value={csrfTag} onchange={(e) => setCsrfTag(e.currentTarget.value)}>
			<option value="Never">{t.settings.server.never}</option>
			<option value="Periodic">{t.settings.server.periodic}</option>
			<option value="AfterRequests">{t.settings.server.afterRequests}</option>
		</select>
	</Field>
	{#if typeof draft.csrf_rotation_policy !== 'string' && 'Periodic' in draft.csrf_rotation_policy}
		<Field label={t.settings.server.rotateEvery}>
			<NumberField
				value={draft.csrf_rotation_policy.Periodic}
				min={0}
				onchange={(v) => v !== '' && (draft.csrf_rotation_policy = { Periodic: v })}
			/>
		</Field>
	{:else if typeof draft.csrf_rotation_policy !== 'string' && 'AfterRequests' in draft.csrf_rotation_policy}
		<Field label={t.settings.server.rotateAfter}>
			<NumberField
				value={draft.csrf_rotation_policy.AfterRequests}
				min={0}
				onchange={(v) => v !== '' && (draft.csrf_rotation_policy = { AfterRequests: v })}
			/>
		</Field>
	{/if}
</SettingsSection>
