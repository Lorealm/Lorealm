<script lang="ts">
	import { connectionStore, type ProfileDraft } from '$lib/stores/connection.svelte.js';
	import {
		PROVIDER_SCHEMAS,
		getProviderSchema,
		OPENROUTER_REASONING_BUDGET
	} from '$lib/types/schema.js';
	import type { JsonValue, Snowflake } from '$lib/types/core.js';
	import NumberParam from './NumberParam.svelte';

	let {
		draft,
		requestSave,
		onManageSecrets
	}: {
		draft: ProfileDraft;
		requestSave: () => void;
		onManageSecrets?: () => void;
	} = $props();

	const schema = $derived(getProviderSchema(draft.provider));
	const c = $derived(schema?.common ?? {});
	const providerSecrets = $derived(connectionStore.secretsFor(draft.provider));

	$effect(() => {
		void connectionStore.loadModels(draft.provider);
	});
	const models = $derived(connectionStore.modelsByProvider.get(draft.provider) ?? []);

	const hasSampling = $derived(!!(c.temperature || c.top_p || c.top_k || c.min_p));
	const hasOutput = $derived(!!(c.max_tokens || c.stop));
	const hasPenalties = $derived(
		!!(c.frequency_penalty || c.presence_penalty || c.repetition_penalty)
	);

	function commonNum(key: string): number | undefined {
		const v = draft.common_params[key];
		return typeof v === 'number' ? v : undefined;
	}
	function setCommon(key: string, v: JsonValue | undefined) {
		if (v === undefined) delete draft.common_params[key];
		else draft.common_params[key] = v;
		requestSave();
	}

	function changeProvider(e: Event) {
		const provider = (e.currentTarget as HTMLSelectElement).value;
		if (provider === draft.provider) return;
		draft.provider = provider;
		draft.provider_params = {};
		// Detach the secret if it doesn't belong to the new provider.
		if (!connectionStore.secretsFor(provider).some((s) => s.id === draft.secret_id)) {
			draft.secret_id = null;
		}
		requestSave();
	}

	function changeSecret(e: Event) {
		const v = (e.currentTarget as HTMLSelectElement).value;
		draft.secret_id = v === '' ? null : (v as Snowflake);
		requestSave();
	}

	const stopText = $derived.by(() => {
		const v = draft.common_params.stop;
		return Array.isArray(v) ? v.filter((s) => typeof s === 'string').join(', ') : '';
	});
	function setStop(e: Event) {
		const arr = (e.currentTarget as HTMLInputElement).value
			.split(',')
			.map((s) => s.trim())
			.filter(Boolean);
		setCommon('stop', arr.length ? arr : undefined);
	}

	const isOpenRouter = $derived(draft.provider === 'openrouter');

	interface Reasoning {
		effort?: string;
		max_tokens?: number;
		enabled?: boolean;
	}
	const reasoning = $derived((draft.provider_params.reasoning ?? {}) as Reasoning);

	const reasoningMode = $derived.by(() => {
		const r = draft.provider_params.reasoning as Reasoning | undefined;
		if (!r) return 'default';
		if (r.enabled === false) return 'off';
		if (typeof r.max_tokens === 'number') return 'budget';
		if (typeof r.effort === 'string') return r.effort;
		return 'default';
	});

	function setReasoning(r: Reasoning | undefined) {
		if (!r || Object.keys(r).length === 0) delete draft.provider_params.reasoning;
		else draft.provider_params.reasoning = r as unknown as JsonValue;
		requestSave();
	}
	function changeReasoningMode(e: Event) {
		const mode = (e.currentTarget as HTMLSelectElement).value;
		if (mode === 'default') setReasoning(undefined);
		else if (mode === 'off') setReasoning({ enabled: false });
		else if (mode === 'budget') setReasoning({ max_tokens: OPENROUTER_REASONING_BUDGET.default });
		else setReasoning({ effort: mode });
	}

	interface Routing {
		order: string[];
		allow_fallbacks?: boolean;
	}
	const routing = $derived((draft.provider_params.provider ?? null) as Routing | null);
	const orderText = $derived(routing?.order?.join(', ') ?? '');
	const fallbacksMode = $derived(
		routing?.allow_fallbacks === true
			? 'yes'
			: routing?.allow_fallbacks === false
				? 'no'
				: 'default'
	);

	function setRouting(order: string[], fallbacks: string) {
		if (order.length === 0 && fallbacks === 'default') {
			delete draft.provider_params.provider;
		} else {
			const r: Routing = { order };
			if (fallbacks !== 'default') r.allow_fallbacks = fallbacks === 'yes';
			draft.provider_params.provider = r as unknown as JsonValue;
		}
		requestSave();
	}
	function changeOrder(e: Event) {
		const order = (e.currentTarget as HTMLInputElement).value
			.split(',')
			.map((s) => s.trim())
			.filter(Boolean);
		setRouting(order, fallbacksMode);
	}
	function changeFallbacks(e: Event) {
		setRouting(routing?.order ?? [], (e.currentTarget as HTMLSelectElement).value);
	}

	// In practice transforms is ["middle-out"] or absent.
	const middleOut = $derived(
		Array.isArray(draft.provider_params.transforms) &&
			(draft.provider_params.transforms as JsonValue[]).includes('middle-out')
	);
	function toggleMiddleOut(e: Event) {
		if ((e.currentTarget as HTMLInputElement).checked) {
			draft.provider_params.transforms = ['middle-out'];
		} else {
			delete draft.provider_params.transforms;
		}
		requestSave();
	}
</script>

<div class="profile-editor">
	<section class="group">
		<h3>Profile</h3>
		<label class="field">
			<span>Name</span>
			<input type="text" bind:value={draft.name} oninput={requestSave} />
		</label>
		<label class="field">
			<span>Provider</span>
			<select value={draft.provider} onchange={changeProvider}>
				{#if !schema}
					<option value={draft.provider}>{draft.provider || '(unset)'}</option>
				{/if}
				{#each PROVIDER_SCHEMAS as s (s.provider)}
					<option value={s.provider}>{s.label}</option>
				{/each}
			</select>
		</label>
		<label class="field">
			<span>Model</span>
			<input type="text" bind:value={draft.model} oninput={requestSave} list="models-{draft.id}" />
			<datalist id="models-{draft.id}">
				{#each models as m (m.id)}
					<option value={m.id} label={m.name ?? ''}></option>
				{/each}
			</datalist>
		</label>
		<label class="field">
			<span>API key</span>
			<select value={draft.secret_id ?? ''} onchange={changeSecret}>
				<option value="">None</option>
				{#each providerSecrets as secret (secret.id)}
					<option value={secret.id}>{secret.label} ({secret.key_hint})</option>
				{/each}
			</select>
		</label>
		{#if providerSecrets.length === 0}
			<p class="hint">
				No keys for this provider.
				{#if onManageSecrets}
					<button class="link" type="button" onclick={onManageSecrets}>Add one</button>
				{/if}
			</p>
		{/if}
	</section>

	{#if !schema}
		<p class="hint">No settings schema for provider “{draft.provider}” — only base fields shown.</p>
	{/if}

	{#if hasSampling}
		<section class="group">
			<h3>Sampling</h3>
			{#if c.temperature}
				<NumberParam
					label="Temperature"
					spec={c.temperature}
					value={commonNum('temperature')}
					onChange={(v) => setCommon('temperature', v)}
				/>
			{/if}
			{#if c.top_p}
				<NumberParam
					label="Top P"
					spec={c.top_p}
					value={commonNum('top_p')}
					onChange={(v) => setCommon('top_p', v)}
				/>
			{/if}
			{#if c.top_k}
				<NumberParam
					label="Top K"
					spec={c.top_k}
					value={commonNum('top_k')}
					onChange={(v) => setCommon('top_k', v)}
				/>
			{/if}
			{#if c.min_p}
				<NumberParam
					label="Min P"
					spec={c.min_p}
					value={commonNum('min_p')}
					onChange={(v) => setCommon('min_p', v)}
				/>
			{/if}
		</section>
	{/if}

	{#if hasOutput}
		<section class="group">
			<h3>Output</h3>
			{#if c.max_tokens}
				<NumberParam
					label="Max tokens"
					spec={c.max_tokens}
					value={commonNum('max_tokens')}
					onChange={(v) => setCommon('max_tokens', v)}
				/>
			{/if}
			{#if c.stop}
				<label class="field">
					<span>Stop sequences <small>(comma-separated)</small></span>
					<input type="text" value={stopText} onchange={setStop} />
				</label>
			{/if}
		</section>
	{/if}

	{#if hasPenalties}
		<section class="group">
			<h3>Penalties</h3>
			{#if c.frequency_penalty}
				<NumberParam
					label="Frequency penalty"
					spec={c.frequency_penalty}
					value={commonNum('frequency_penalty')}
					onChange={(v) => setCommon('frequency_penalty', v)}
				/>
			{/if}
			{#if c.presence_penalty}
				<NumberParam
					label="Presence penalty"
					spec={c.presence_penalty}
					value={commonNum('presence_penalty')}
					onChange={(v) => setCommon('presence_penalty', v)}
				/>
			{/if}
			{#if c.repetition_penalty}
				<NumberParam
					label="Repetition penalty"
					spec={c.repetition_penalty}
					value={commonNum('repetition_penalty')}
					onChange={(v) => setCommon('repetition_penalty', v)}
				/>
			{/if}
		</section>
	{/if}

	{#if isOpenRouter}
		<section class="group">
			<h3>OpenRouter</h3>
			<label class="field">
				<span>Reasoning</span>
				<select value={reasoningMode} onchange={changeReasoningMode}>
					<option value="default">Model default</option>
					<option value="off">Disabled</option>
					<option value="low">Low effort</option>
					<option value="medium">Medium effort</option>
					<option value="high">High effort</option>
					<option value="budget">Token budget</option>
				</select>
			</label>
			{#if reasoningMode === 'budget'}
				<NumberParam
					label="Reasoning budget"
					spec={OPENROUTER_REASONING_BUDGET}
					value={reasoning.max_tokens}
					onChange={(v) => setReasoning(v === undefined ? undefined : { max_tokens: v })}
				/>
			{/if}
			<label class="field">
				<span>Provider order <small>(comma-separated, e.g. anthropic, deepinfra)</small></span>
				<input type="text" value={orderText} onchange={changeOrder} />
			</label>
			<label class="field">
				<span>Fallbacks</span>
				<select value={fallbacksMode} onchange={changeFallbacks}>
					<option value="default">OpenRouter default</option>
					<option value="yes">Allowed</option>
					<option value="no">Only listed providers</option>
				</select>
			</label>
			<label class="check">
				<input type="checkbox" checked={middleOut} onchange={toggleMiddleOut} />
				<span>Middle-out compression</span>
			</label>
		</section>
	{/if}
</div>

<style>
	.profile-editor {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
		padding: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		max-width: 560px;
	}
	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding-bottom: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.group:last-child {
		border-bottom: none;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
	}
	.check {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		font-size: var(--font-size-sm);
	}
	.hint {
		margin: 0;
		font-size: var(--font-size-sm);
		opacity: 0.7;
	}
	.link {
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		text-decoration: underline;
		font-size: var(--font-size-sm);
	}
</style>
