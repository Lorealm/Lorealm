<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { connectionStore, type ProfileDraft } from '#lib/stores/connection.svelte.js';
	import {
		ANTHROPIC_THINKING_BUDGET,
		PROVIDER_SCHEMAS,
		VERTEX_THINKING_BUDGET,
		getProviderSchema,
		INSTRUCT_PRESETS,
		type InstructTemplate,
		OPENROUTER_REASONING_BUDGET,
		TOKEN_BUDGET,
		TOOL_ROUNDS
	} from '#lib/types/schema.js';
	import type { JsonValue, Snowflake } from '#lib/types/core.js';
	import NumberParam from './NumberParam.svelte';
	import SearchSelect from '#lib/components/SearchSelect.svelte';
	import { parse as parseYaml } from 'yaml';

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
		void connectionStore.loadModels(draft.id);
	});
	const models = $derived(connectionStore.modelsByProfile.get(draft.id));
	const modelOptions = $derived(
		(models ?? []).map((m) => ({
			value: m.id,
			label: m.name ?? m.id,
			hint: m.name ? m.id : undefined
		}))
	);

	const modelEmptyText = $derived(
		models === undefined ? t.connections.editor.loadingModels : t.connections.editor.noModels
	);

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

	// Saved as the YAML the user typed; the server parses it per request.
	let yamlErrors = $state<Record<string, boolean>>({});
	function setYaml(key: string, shape: 'mapping' | 'list', e: Event) {
		const text = (e.currentTarget as HTMLTextAreaElement).value;
		let ok: boolean;
		try {
			const v: unknown = parseYaml(text);
			ok =
				v == null ||
				(shape === 'list' ? Array.isArray(v) : typeof v === 'object' && !Array.isArray(v));
		} catch {
			ok = false;
		}
		yamlErrors[key] = !ok;
		if (ok) setCommon(key, text.trim() === '' ? undefined : text);
	}

	const isOpenRouter = $derived(draft.provider === 'openrouter');
	const isOpenAi = $derived(draft.provider === 'openai');
	const isAnthropic = $derived(draft.provider === 'anthropic');
	const isVertex = $derived(draft.provider === 'vertex');
	const isOpenAiPlatform = $derived(draft.provider === 'openai_platform');
	const isGemini = $derived(draft.provider === 'gemini');
	const BASE_URL_PLACEHOLDERS: Record<string, string> = {
		anthropic: 'https://api.anthropic.com/v1',
		gemini: 'https://generativelanguage.googleapis.com/v1beta',
		nanogpt: 'https://nano-gpt.com/api/v1',
		chutes: 'https://llm.chutes.ai/v1',
		deepseek: 'https://api.deepseek.com',
		zai: 'https://api.z.ai/api/paas/v4',
		moonshot: 'https://api.moonshot.ai/v1',
		xai: 'https://api.x.ai/v1'
	};
	// Fixed-endpoint OpenAI-compatible services; the base URL only matters for a proxy.
	const isHostedOpenAi = $derived(
		isOpenAiPlatform ||
			['nanogpt', 'chutes', 'deepseek', 'zai', 'moonshot', 'xai'].includes(draft.provider)
	);

	function textParam(key: string): string {
		const v = draft.provider_params[key];
		return typeof v === 'string' ? v : '';
	}
	function setTextParam(key: string, e: Event) {
		const v = (e.currentTarget as HTMLInputElement | HTMLSelectElement).value.trim();
		if (v === '') delete draft.provider_params[key];
		else draft.provider_params[key] = v;
		requestSave();
	}

	const template = $derived(
		draft.provider_params.template as unknown as InstructTemplate | undefined
	);
	const templatePreset = $derived.by(() => {
		const current = template;
		if (!current) return 'off';
		const match = INSTRUCT_PRESETS.find((p) =>
			(Object.keys(p.template) as (keyof InstructTemplate)[]).every(
				(k) => p.template[k] === current[k]
			)
		);
		return match?.name ?? 'custom';
	});
	const templateFields = $derived([
		['system_prefix', t.connections.editor.systemPrefix],
		['system_suffix', t.connections.editor.systemSuffix],
		['user_prefix', t.connections.editor.userPrefix],
		['user_suffix', t.connections.editor.userSuffix],
		['assistant_prefix', t.connections.editor.assistantPrefix],
		['assistant_suffix', t.connections.editor.assistantSuffix]
	] as const);
	function setTemplate(t: InstructTemplate | undefined) {
		if (t) draft.provider_params.template = { ...t } as unknown as JsonValue;
		else delete draft.provider_params.template;
		requestSave();
	}
	function changeTemplatePreset(e: Event) {
		const name = (e.currentTarget as HTMLSelectElement).value;
		setTemplate(INSTRUCT_PRESETS.find((p) => p.name === name)?.template);
	}

	// Recent Claude models refuse requests that set these.
	const anthropicSamplingSet = $derived(
		isAnthropic && ['temperature', 'top_p', 'top_k'].some((k) => commonNum(k) !== undefined)
	);

	// Mirrors the serde tagging of `AnthropicThinking` on the server.
	type AnthropicThinking =
		| { type: 'adaptive'; display?: string }
		| { type: 'enabled'; budget_tokens: number }
		| { type: 'disabled' };

	const anthropicThinking = $derived(
		draft.provider_params.thinking as unknown as AnthropicThinking | undefined
	);
	const anthropicThinkingMode = $derived.by(() => {
		const t = anthropicThinking;
		if (!t) return 'default';
		if (t.type === 'adaptive') return t.display === 'summarized' ? 'summarized' : 'hidden';
		return t.type === 'enabled' ? 'budget' : 'off';
	});
	function setAnthropicThinking(t: AnthropicThinking | undefined) {
		if (t) draft.provider_params.thinking = t as unknown as JsonValue;
		else delete draft.provider_params.thinking;
		requestSave();
	}
	function changeAnthropicThinking(e: Event) {
		const mode = (e.currentTarget as HTMLSelectElement).value;
		if (mode === 'summarized') setAnthropicThinking({ type: 'adaptive', display: 'summarized' });
		else if (mode === 'hidden') setAnthropicThinking({ type: 'adaptive' });
		else if (mode === 'off') setAnthropicThinking({ type: 'disabled' });
		else if (mode === 'budget') {
			setAnthropicThinking({
				type: 'enabled',
				budget_tokens: ANTHROPIC_THINKING_BUDGET.default
			});
		} else setAnthropicThinking(undefined);
	}

	interface VertexThinking {
		thinking_level?: string;
		thinking_budget?: number;
		include_thoughts?: boolean;
	}
	const vertexThinking = $derived(
		(draft.provider_params.thinking ?? {}) as unknown as VertexThinking
	);
	const vertexThinkingMode = $derived(
		typeof vertexThinking.thinking_budget === 'number'
			? 'budget'
			: (vertexThinking.thinking_level ?? 'default')
	);
	function setVertexThinking(t: VertexThinking) {
		if (Object.keys(t).length === 0) delete draft.provider_params.thinking;
		else draft.provider_params.thinking = t as unknown as JsonValue;
		requestSave();
	}
	// The level and the budget are alternatives; a model takes one or the other.
	function changeVertexThinking(e: Event) {
		const mode = (e.currentTarget as HTMLSelectElement).value;
		const t: VertexThinking = {};
		if (vertexThinking.include_thoughts) t.include_thoughts = true;
		if (mode === 'budget') t.thinking_budget = VERTEX_THINKING_BUDGET.default;
		else if (mode !== 'default') t.thinking_level = mode;
		setVertexThinking(t);
	}
	function setVertexBudget(budget: number | undefined) {
		const t = { ...vertexThinking };
		if (budget === undefined) delete t.thinking_budget;
		else t.thinking_budget = budget;
		setVertexThinking(t);
	}
	function toggleVertexThoughts(e: Event) {
		const t = { ...vertexThinking };
		if ((e.currentTarget as HTMLInputElement).checked) t.include_thoughts = true;
		else delete t.include_thoughts;
		setVertexThinking(t);
	}

	const VERTEX_SAFETY_CATEGORIES = [
		'HARM_CATEGORY_HARASSMENT',
		'HARM_CATEGORY_HATE_SPEECH',
		'HARM_CATEGORY_SEXUALLY_EXPLICIT',
		'HARM_CATEGORY_DANGEROUS_CONTENT'
	];
	// One threshold across every category: the editor offers no per-category control.
	const vertexSafety = $derived.by(() => {
		const settings = draft.provider_params.safety_settings;
		const first = Array.isArray(settings) ? settings[0] : undefined;
		return (first as { threshold?: string } | undefined)?.threshold ?? 'default';
	});
	function changeVertexSafety(e: Event) {
		const threshold = (e.currentTarget as HTMLSelectElement).value;
		if (threshold === 'default') delete draft.provider_params.safety_settings;
		else {
			draft.provider_params.safety_settings = VERTEX_SAFETY_CATEGORIES.map((category) => ({
				category,
				threshold
			}));
		}
		requestSave();
	}

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
		<h3>{t.connections.editor.profile}</h3>
		<label class="field">
			<span>{t.connections.editor.name}</span>
			<input type="text" bind:value={draft.name} oninput={requestSave} />
		</label>
		<label class="field">
			<span>{t.connections.editor.provider}</span>
			<select value={draft.provider} onchange={changeProvider}>
				{#if !schema}
					<option value={draft.provider}>{draft.provider || t.connections.editor.unset}</option>
				{/if}
				{#each PROVIDER_SCHEMAS as s (s.provider)}
					<option value={s.provider}>{s.label}</option>
				{/each}
			</select>
		</label>
		<div class="field">
			<span>{t.connections.editor.model}</span>
			<SearchSelect
				value={draft.model}
				options={modelOptions}
				onchange={(v) => {
					draft.model = v;
					requestSave();
				}}
				placeholder={t.connections.editor.modelPlaceholder}
				emptyText={modelEmptyText}
				allowCustom
				ariaLabel={t.connections.editor.model}
			/>
		</div>
		<label class="field">
			<span>{t.connections.editor.apiKey}</span>
			<select value={draft.secret_id ?? ''} onchange={changeSecret}>
				<option value="">{t.connections.editor.none}</option>
				{#each providerSecrets as secret (secret.id)}
					<option value={secret.id}>{secret.label} ({secret.key_hint})</option>
				{/each}
			</select>
		</label>
		{#if providerSecrets.length === 0}
			<p class="hint">
				{t.connections.editor.noKeys}
				{#if onManageSecrets}
					<button class="link" type="button" onclick={onManageSecrets}
						>{t.connections.editor.addOne}</button
					>
				{/if}
			</p>
		{/if}
		<NumberParam
			label={t.connections.editor.tokenBudget}
			spec={TOKEN_BUDGET}
			value={draft.token_budget}
			onChange={(v) => {
				draft.token_budget = v ?? TOKEN_BUDGET.default;
				requestSave();
			}}
		/>
		<NumberParam
			label={t.connections.editor.toolRounds}
			spec={TOOL_ROUNDS}
			value={commonNum('max_tool_rounds')}
			onChange={(v) => setCommon('max_tool_rounds', v)}
		/>
		<label class="check">
			<input
				type="checkbox"
				checked={draft.common_params.squash_messages === true}
				onchange={(e) => setCommon('squash_messages', e.currentTarget.checked || undefined)}
			/>
			<span>{t.connections.editor.squashMessages}</span>
		</label>
	</section>

	{#if isOpenAi || isAnthropic || isHostedOpenAi || isGemini}
		<section class="group">
			<h3>{t.connections.editor.endpoint}</h3>
			<label class="field">
				<span>{t.connections.editor.baseUrl}</span>
				<input
					type="text"
					value={textParam('base_url')}
					placeholder={BASE_URL_PLACEHOLDERS[draft.provider] ?? 'https://api.openai.com/v1'}
					onchange={(e) => setTextParam('base_url', e)}
				/>
			</label>
			{#if isOpenAi}
				<label class="field">
					<span>{t.connections.editor.textCompletion}</span>
					<select value={templatePreset} onchange={changeTemplatePreset}>
						<option value="off">{t.connections.editor.textCompletionOff}</option>
						{#each INSTRUCT_PRESETS as preset (preset.name)}
							<option value={preset.name}>{preset.name}</option>
						{/each}
						{#if templatePreset === 'custom'}
							<option value="custom">{t.connections.editor.custom}</option>
						{/if}
					</select>
				</label>
			{/if}
		</section>
	{/if}

	{#if isOpenAi && template}
		<section class="group">
			<h3>{t.connections.editor.instructTemplate}</h3>
			<div class="template-grid">
				{#each templateFields as [key, label] (key)}
					<label class="field">
						<span>{label}</span>
						<textarea
							rows="2"
							value={template[key]}
							onchange={(e) => setTemplate({ ...template, [key]: e.currentTarget.value })}
						></textarea>
					</label>
				{/each}
			</div>
			<p class="hint">{t.connections.editor.instructTemplateHint}</p>
		</section>
	{/if}

	{#if isVertex}
		<section class="group">
			<h3>Google Cloud</h3>
			<label class="field">
				<span>{t.connections.editor.projectId}</span>
				<input
					type="text"
					value={textParam('project_id')}
					placeholder="my-project"
					onchange={(e) => setTextParam('project_id', e)}
				/>
			</label>
			<label class="field">
				<span>{t.connections.editor.location}</span>
				<input
					type="text"
					value={textParam('location')}
					placeholder="global"
					onchange={(e) => setTextParam('location', e)}
				/>
			</label>
			<p class="hint">{t.connections.editor.projectHint}</p>
		</section>
	{/if}

	{#if !schema}
		<p class="hint">{t.connections.editor.noSchema({ provider: draft.provider })}</p>
	{/if}

	{#if hasSampling}
		<section class="group">
			<h3>{t.connections.editor.sampling}</h3>
			{#if c.temperature}
				<NumberParam
					label={t.connections.editor.temperature}
					spec={c.temperature}
					value={commonNum('temperature')}
					onChange={(v) => setCommon('temperature', v)}
				/>
			{/if}
			{#if c.top_p}
				<NumberParam
					label={t.connections.editor.topP}
					spec={c.top_p}
					value={commonNum('top_p')}
					onChange={(v) => setCommon('top_p', v)}
				/>
			{/if}
			{#if c.top_k}
				<NumberParam
					label={t.connections.editor.topK}
					spec={c.top_k}
					value={commonNum('top_k')}
					onChange={(v) => setCommon('top_k', v)}
				/>
			{/if}
			{#if c.min_p}
				<NumberParam
					label={t.connections.editor.minP}
					spec={c.min_p}
					value={commonNum('min_p')}
					onChange={(v) => setCommon('min_p', v)}
				/>
			{/if}
			{#if anthropicSamplingSet}
				<p class="hint">{t.connections.editor.claudeSamplingHint}</p>
			{/if}
		</section>
	{/if}

	{#if hasOutput}
		<section class="group">
			<h3>{t.connections.editor.output}</h3>
			{#if c.max_tokens}
				<NumberParam
					label={t.connections.editor.maxTokens}
					spec={c.max_tokens}
					value={commonNum('max_tokens')}
					onChange={(v) => setCommon('max_tokens', v)}
				/>
			{/if}
			{#if c.stop}
				<label class="field">
					<span
						>{t.connections.editor.stopSequences}
						<small>{t.connections.editor.commaSeparated}</small></span
					>
					<input type="text" value={stopText} onchange={setStop} />
				</label>
			{/if}
		</section>
	{/if}

	{#if hasPenalties}
		<section class="group">
			<h3>{t.connections.editor.penalties}</h3>
			{#if c.frequency_penalty}
				<NumberParam
					label={t.connections.editor.frequencyPenalty}
					spec={c.frequency_penalty}
					value={commonNum('frequency_penalty')}
					onChange={(v) => setCommon('frequency_penalty', v)}
				/>
			{/if}
			{#if c.presence_penalty}
				<NumberParam
					label={t.connections.editor.presencePenalty}
					spec={c.presence_penalty}
					value={commonNum('presence_penalty')}
					onChange={(v) => setCommon('presence_penalty', v)}
				/>
			{/if}
			{#if c.repetition_penalty}
				<NumberParam
					label={t.connections.editor.repetitionPenalty}
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
				<span>{t.connections.editor.reasoning}</span>
				<select value={reasoningMode} onchange={changeReasoningMode}>
					<option value="default">{t.connections.editor.modelDefault}</option>
					<option value="off">{t.connections.editor.disabled}</option>
					<option value="low">{t.connections.editor.lowEffort}</option>
					<option value="medium">{t.connections.editor.mediumEffort}</option>
					<option value="high">{t.connections.editor.highEffort}</option>
					<option value="budget">{t.connections.editor.tokenBudgetOption}</option>
				</select>
			</label>
			{#if reasoningMode === 'budget'}
				<NumberParam
					label={t.connections.editor.reasoningBudget}
					spec={OPENROUTER_REASONING_BUDGET}
					value={reasoning.max_tokens}
					onChange={(v) => setReasoning(v === undefined ? undefined : { max_tokens: v })}
				/>
			{/if}
			<label class="field">
				<span
					>{t.connections.editor.providerOrder}
					<small>{t.connections.editor.providerOrderHint}</small></span
				>
				<input type="text" value={orderText} onchange={changeOrder} />
			</label>
			<label class="field">
				<span>{t.connections.editor.fallbacks}</span>
				<select value={fallbacksMode} onchange={changeFallbacks}>
					<option value="default">{t.connections.editor.openRouterDefault}</option>
					<option value="yes">{t.connections.editor.allowed}</option>
					<option value="no">{t.connections.editor.onlyListed}</option>
				</select>
			</label>
			<label class="check">
				<input type="checkbox" checked={middleOut} onchange={toggleMiddleOut} />
				<span>{t.connections.editor.middleOut}</span>
			</label>
		</section>
	{/if}

	{#if isAnthropic}
		<section class="group">
			<h3>Anthropic</h3>
			<label class="field">
				<span>{t.connections.editor.thinking}</span>
				<select value={anthropicThinkingMode} onchange={changeAnthropicThinking}>
					<option value="default">{t.connections.editor.modelDefault}</option>
					<option value="summarized">{t.connections.editor.showSummaries}</option>
					<option value="hidden">{t.connections.editor.hidden}</option>
					<option value="off">{t.connections.editor.off}</option>
					<option value="budget">{t.connections.editor.budgetOlder}</option>
				</select>
			</label>
			{#if anthropicThinkingMode === 'off'}
				<p class="hint">{t.connections.editor.cantDisableThinking}</p>
			{/if}
			{#if anthropicThinking?.type === 'enabled'}
				<NumberParam
					label={t.connections.editor.thinkingBudget}
					spec={ANTHROPIC_THINKING_BUDGET}
					value={anthropicThinking.budget_tokens}
					onChange={(v) =>
						setAnthropicThinking(
							v === undefined ? undefined : { type: 'enabled', budget_tokens: v }
						)}
				/>
			{/if}
			<label class="field">
				<span>{t.connections.editor.effort}</span>
				<select value={textParam('effort')} onchange={(e) => setTextParam('effort', e)}>
					<option value="">{t.connections.editor.modelDefault}</option>
					<option value="low">{t.connections.editor.low}</option>
					<option value="medium">{t.connections.editor.medium}</option>
					<option value="high">{t.connections.editor.high}</option>
					<option value="xhigh">{t.connections.editor.extraHigh}</option>
					<option value="max">{t.connections.editor.max}</option>
				</select>
			</label>
		</section>
	{/if}

	{#if isOpenAiPlatform}
		<section class="group">
			<h3>OpenAI</h3>
			<label class="field">
				<span>{t.connections.editor.reasoning}</span>
				<select
					value={textParam('reasoning_effort')}
					onchange={(e) => setTextParam('reasoning_effort', e)}
				>
					<option value="">{t.connections.editor.modelDefault}</option>
					<option value="none">{t.connections.editor.off}</option>
					<option value="low">{t.connections.editor.low}</option>
					<option value="medium">{t.connections.editor.medium}</option>
					<option value="high">{t.connections.editor.high}</option>
					<option value="xhigh">{t.connections.editor.extraHigh}</option>
				</select>
			</label>
		</section>
	{/if}

	{#if isVertex || isGemini}
		<section class="group">
			<h3>Gemini</h3>
			<label class="field">
				<span>{t.connections.editor.thinking}</span>
				<select value={vertexThinkingMode} onchange={changeVertexThinking}>
					<option value="default">{t.connections.editor.modelDefault}</option>
					<option value="low">{t.connections.editor.low}</option>
					<option value="high">{t.connections.editor.high}</option>
					<option value="budget">{t.connections.editor.budgetGemini}</option>
				</select>
			</label>
			{#if vertexThinkingMode === 'budget'}
				<NumberParam
					label={t.connections.editor.thinkingBudget}
					spec={VERTEX_THINKING_BUDGET}
					value={vertexThinking.thinking_budget}
					onChange={setVertexBudget}
				/>
			{/if}
			<label class="check">
				<input
					type="checkbox"
					checked={vertexThinking.include_thoughts === true}
					onchange={toggleVertexThoughts}
				/>
				<span>{t.connections.editor.showThoughts}</span>
			</label>
			<label class="field">
				<span>{t.connections.editor.safetyFilter}</span>
				<select value={vertexSafety} onchange={changeVertexSafety}>
					<option value="default">{t.connections.editor.googleDefault}</option>
					<option value="BLOCK_NONE">{t.connections.editor.off}</option>
					<option value="BLOCK_ONLY_HIGH">{t.connections.editor.blockHigh}</option>
					<option value="BLOCK_MEDIUM_AND_ABOVE">{t.connections.editor.blockMedium}</option>
					<option value="BLOCK_LOW_AND_ABOVE">{t.connections.editor.blockLow}</option>
				</select>
			</label>
		</section>
	{/if}

	{#snippet yamlField(
		key: string,
		shape: 'mapping' | 'list',
		label: string,
		hint: string,
		placeholder: string
	)}
		<label class="field">
			<span>{label}</span>
			<textarea
				rows="3"
				value={typeof draft.common_params[key] === 'string' ? draft.common_params[key] : ''}
				{placeholder}
				onchange={(e) => setYaml(key, shape, e)}
			></textarea>
		</label>
		{#if yamlErrors[key]}
			<p class="error">
				{shape === 'list' ? t.connections.editor.yamlList : t.connections.editor.yamlMapping}
			</p>
		{:else}
			<p class="hint">{hint}</p>
		{/if}
	{/snippet}

	<section class="group">
		<h3>{t.connections.editor.customFields}</h3>
		{@render yamlField(
			'include_body',
			'mapping',
			t.connections.editor.includeBody,
			t.connections.editor.includeBodyHint,
			'mirostat: 2'
		)}
		{@render yamlField(
			'exclude_body',
			'list',
			t.connections.editor.excludeBody,
			t.connections.editor.excludeBodyHint,
			'- frequency_penalty'
		)}
		{@render yamlField(
			'include_headers',
			'mapping',
			t.connections.editor.includeHeaders,
			t.connections.editor.includeHeadersHint,
			'X-Custom-Header: value'
		)}
	</section>
</div>

<style>
	.profile-editor {
		flex: 1 1 auto;
		width: 100%;
		overflow-y: auto;
		padding: var(--spacing-md);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
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
	.field > span {
		color: var(--color-text-secondary);
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
	.template-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: var(--spacing-sm);
	}
	textarea {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		resize: vertical;
	}
	.error {
		margin: 0;
		font-size: var(--font-size-sm);
		color: var(--color-text-error, #c0392b);
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
