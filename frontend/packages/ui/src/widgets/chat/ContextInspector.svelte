<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { tick } from 'svelte';
	import { useApi } from '#lib/api/client.js';
	import { contextInspector } from '#lib/stores/contextInspector.svelte.js';
	import { keyLayer } from '#lib/keys/layers.svelte.js';
	import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
	import { showApiError } from '#lib/stores/errorActions.js';
	import {
		parseContextBlocks,
		hasStoredContext,
		groupAdjacentBlocks,
		parseUsageReport,
		sourceOf,
		rankSources,
		countsTotal,
		SOURCE_LABELS,
		type SourceCounts,
		type TracedBlock
	} from '#lib/types/context.js';
	import { resolveBlockContent } from '#lib/types/content.js';
	import type { JsonValue, Snowflake } from '#lib/types/core.js';
	import { ChevronDown, Info, X } from '@lucide/svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	const api = useApi();

	function metaString(meta: JsonValue, key: string): string | null {
		if (meta && typeof meta === 'object' && !Array.isArray(meta)) {
			const v = (meta as Record<string, JsonValue>)[key];
			return typeof v === 'string' ? v : null;
		}
		return null;
	}

	function readCount(counts: unknown, key: string): number | null {
		if (counts instanceof Map) {
			const v = counts.get(key);
			return typeof v === 'number' ? v : null;
		}
		if (counts && typeof counts === 'object') {
			const v = (counts as Record<string, unknown>)[key];
			return typeof v === 'number' ? v : null;
		}
		return null;
	}

	function roleClass(role: string): 'system' | 'user' | 'assistant' | 'unknown' {
		return role === 'system' || role === 'user' || role === 'assistant' ? role : 'unknown';
	}

	function fmtCount(n: number): string {
		return n.toLocaleString();
	}

	function fmtValue(value: unknown): string {
		if (typeof value === 'string') return value;
		if (value === null || value === undefined) return String(value);
		if (typeof value === 'object') {
			try {
				return JSON.stringify(value);
			} catch {
				return String(value);
			}
		}
		return String(value);
	}

	function fmtJson(value: unknown): string {
		if (typeof value === 'string') return value;
		if (value === null || value === undefined) return String(value);
		try {
			return JSON.stringify(value, null, 2);
		} catch {
			return String(value);
		}
	}

	function fmtDuration(ms: number): string {
		return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(2)} s`;
	}

	// Messages only flag that a prompt was kept; it's fetched when the inspector opens.
	let fetched = $state<{ id: Snowflake; blocks: TracedBlock[] | null } | null>(null);
	let loadingContext = $derived(
		contextInspector.message !== null && fetched?.id !== contextInspector.message.id
	);
	$effect(() => {
		const msg = contextInspector.message;
		if (!msg) return;
		const msgId = msg.id;
		if (!hasStoredContext(msg.metadata)) {
			fetched = { id: msgId, blocks: null };
			return;
		}
		api.chat.messages
			.context(msg.conversation_id, msgId)
			.then((res) => {
				if (contextInspector.message?.id === msgId)
					fetched = { id: msgId, blocks: parseContextBlocks(res) };
			})
			.catch((e) => {
				if (contextInspector.message?.id !== msgId) return;
				fetched = { id: msgId, blocks: null };
				showApiError(e);
			});
	});
	let blocks = $derived(
		fetched && fetched.id === contextInspector.message?.id ? fetched.blocks : null
	);
	let groups = $derived(blocks ? groupAdjacentBlocks(blocks) : null);

	let flat = $derived.by(() => {
		const out: { block: TracedBlock; gi: number }[] = [];
		(groups ?? []).forEach((g, gi) => g.blocks.forEach((block) => out.push({ block, gi })));
		return out;
	});
	let flatTexts = $derived(flat.map((f) => resolveBlockContent(f.block).text));

	let groupOffsets = $derived.by(() => {
		const offs: number[] = [];
		let acc = 0;
		(groups ?? []).forEach((g, gi) => {
			offs[gi] = acc;
			acc += g.blocks.length;
		});
		return offs;
	});

	type SizeData = { unit: 'tokens' | 'chars'; counts: number[]; total: number };

	let tokenData = $state<SizeData | null>(null);

	let display = $derived.by<SizeData>(() => {
		if (tokenData) return tokenData;
		const counts = flatTexts.map((t) => t.length);
		return { unit: 'chars', counts, total: counts.reduce((a, b) => a + b, 0) };
	});

	let groupTotals = $derived.by(() => {
		const m = new SvelteMap<number, number>();
		flat.forEach((f, i) => m.set(f.gi, (m.get(f.gi) ?? 0) + (display.counts[i] ?? 0)));
		return m;
	});

	let sourceTotals = $derived.by<SourceCounts>(() => {
		const out: SourceCounts = {};
		flat.forEach((f, i) => {
			const key = sourceOf(f.block.origin);
			out[key] = (out[key] ?? 0) + (display.counts[i] ?? 0);
		});
		return out;
	});
	let sourceRows = $derived(rankSources(sourceTotals));

	let recorded = $derived(
		contextInspector.message ? parseUsageReport(contextInspector.message.metadata) : null
	);

	let trimmedRows = $derived(recorded ? rankSources(recorded.trimmed) : []);
	let trimmedMax = $derived(trimmedRows.reduce((m, [, n]) => Math.max(m, n), 0));

	function pct(n: number, of: number): number {
		return of > 0 ? Math.min(100, (n / of) * 100) : 0;
	}

	let drift = $derived.by(() => {
		if (!recorded || display.unit !== 'tokens') return null;
		const then = countsTotal(recorded.kept);
		if (then === 0) return null;
		const gap = Math.abs(then - display.total) / then;
		return gap > 0.02 ? then : null;
	});

	$effect(() => {
		const msg = contextInspector.message;
		const texts = flatTexts;
		tokenData = null;
		if (!msg || texts.length === 0) return;

		const tokId = metaString(msg.metadata, 'tok_id');
		if (!tokId) return; // no tokenizer recorded — stay on char fallback

		const msgId = msg.id;
		const batch: Record<string, string> = {};
		texts.forEach((t, i) => (batch[`b${i}`] = t));
		const req = { batch } as unknown as Parameters<typeof api.tokenizers.count.batch>[1];

		api.tokenizers.count
			.batch(tokId, req)
			.then((res) => {
				if (contextInspector.message?.id !== msgId) return; // stale
				const counts = texts.map((_, i) => readCount(res.counts, `b${i}`));
				if (counts.every((c) => c === null)) return; // nothing usable — keep chars
				const norm = counts.map((c) => c ?? 0);
				const total =
					typeof res.total_tokens === 'number' ? res.total_tokens : norm.reduce((a, b) => a + b, 0);
				tokenData = { unit: 'tokens', counts: norm, total };
			})
			.catch((e) => showApiError(e, { title: t.chat.inspector.tokenCountFailed }));
	});

	let templateName = $state<string | null>(null);
	let templateDesc = $state<string | null>(null);
	let profileName = $state<string | null>(null);
	let modelName = $state<string | null>(null);
	let profileBudget = $state<number | null>(null);
	let profileParams = $state<Record<string, JsonValue>>({});
	let profileFailed = $state(false);

	$effect(() => {
		const msg = contextInspector.message;
		templateName = null;
		templateDesc = null;
		profileName = null;
		modelName = null;
		profileBudget = null;
		profileParams = {};
		profileFailed = false;
		if (!msg) return;

		const msgId = msg.id;
		const tid = metaString(msg.metadata, 'template_id');
		const pid = metaString(msg.metadata, 'conn_profile');

		if (tid)
			api.templates
				.get(tid)
				.then((t) => {
					if (contextInspector.message?.id === msgId) {
						templateName = t.name;
						templateDesc = t.description || null;
					}
				})
				.catch(() => {});
		if (pid)
			api.connection.profiles
				.get(pid)
				.then((p) => {
					if (contextInspector.message?.id === msgId) {
						profileName = p.name;
						modelName = p.model;
						profileBudget = p.token_budget;
						profileParams =
							p.common_params &&
							typeof p.common_params === 'object' &&
							!Array.isArray(p.common_params)
								? (p.common_params as Record<string, JsonValue>)
								: {};
					}
				})
				.catch(() => {
					if (contextInspector.message?.id === msgId) profileFailed = true;
				});
	});

	const sampling = (): [string, string][] => [
		['temperature', t.chat.inspector.sampling.temperature],
		['top_p', t.chat.inspector.sampling.topP],
		['top_k', t.chat.inspector.sampling.topK],
		['min_p', t.chat.inspector.sampling.minP]
	];

	let samplingRows = $derived(
		sampling().map(([key, label]) => {
			const v = profileParams[key];
			return { label, value: typeof v === 'number' ? String(v) : '—' };
		})
	);

	let budgetShown = $derived(recorded?.budget ?? profileBudget);
	let budgetIsLive = $derived(recorded?.budget == null && profileBudget !== null);
	let hintReady = $derived(templateName !== null || profileName !== null || profileFailed);

	type Recon = {
		activation: Record<
			string,
			{
				current_turn: number;
				last_activated: Record<string, number>;
				sticky_remaining: Record<string, number>;
			}
		>;
		persistent: Record<string, Record<string, JsonValue>>;
	};

	let recon = $state<Recon | null>(null);
	let reconLoading = $state(false);

	$effect(() => {
		const msg = contextInspector.message;
		recon = null;
		reconLoading = false;
		if (!msg) return;

		const msgId = msg.id;
		reconLoading = true;
		api.chat.messages
			.state(msg.conversation_id, msg.id)
			.then((s) => {
				if (contextInspector.message?.id !== msgId) return;
				recon = s as Recon;
				reconLoading = false;
			})
			.catch((e) => {
				if (contextInspector.message?.id !== msgId) return;
				reconLoading = false;
				showApiError(e, { title: t.chat.inspector.stateFailed });
			});
	});

	let activationGroups = $derived.by(() => {
		if (!recon) return [];
		return Object.entries(recon.activation).map(([id, act]) => {
			const ids = new Set([
				...Object.keys(act.last_activated),
				...Object.keys(act.sticky_remaining)
			]);
			const entries = [...ids]
				.map((entryId) => ({
					entryId,
					lastActivated: act.last_activated[entryId],
					sticky: act.sticky_remaining[entryId]
				}))
				.sort((a, b) => a.entryId.localeCompare(b.entryId));
			return { id, currentTurn: act.current_turn, entries };
		});
	});

	let persistentPools = $derived.by(() => {
		if (!recon) return [];
		return Object.entries(recon.persistent).map(([name, pool]) => ({
			name,
			entries: Object.entries(pool).map(([key, value]) => ({ key, value }))
		}));
	});

	type ToolCall = {
		round: number;
		id: string;
		name: string;
		input: unknown;
		output: unknown;
		isError: boolean;
		durationMs: number;
	};

	function parseToolCalls(meta: JsonValue): ToolCall[] {
		if (!meta || typeof meta !== 'object' || Array.isArray(meta)) return [];
		const raw = (meta as Record<string, JsonValue>).tool_calls;
		if (!Array.isArray(raw)) return [];
		return raw.map((r) => {
			const o =
				r && typeof r === 'object' && !Array.isArray(r) ? (r as Record<string, JsonValue>) : {};
			return {
				round: typeof o.round === 'number' ? o.round : 0,
				id: typeof o.id === 'string' ? o.id : '',
				name: typeof o.name === 'string' ? o.name : t.chat.inspector.unknownTool,
				input: o.input,
				output: o.output,
				isError: o.is_error === true,
				durationMs: typeof o.duration_ms === 'number' ? o.duration_ms : 0
			};
		});
	}

	let toolCalls = $derived(
		contextInspector.message ? parseToolCalls(contextInspector.message.metadata) : []
	);

	/** The tool names the model was offered this turn, whether or not it used them. */
	function parseAvailableTools(meta: JsonValue): string[] | null {
		if (!meta || typeof meta !== 'object' || Array.isArray(meta)) return null;
		const raw = (meta as Record<string, JsonValue>).available_tools;
		if (!Array.isArray(raw)) return null;
		return raw.filter((v): v is string => typeof v === 'string');
	}

	let availableTools = $derived(
		contextInspector.message ? parseAvailableTools(contextInspector.message.metadata) : null
	);

	let toolRounds = $derived.by(() => {
		const m = new SvelteMap<number, ToolCall[]>();
		for (const tc of toolCalls) {
			const arr = m.get(tc.round) ?? [];
			arr.push(tc);
			m.set(tc.round, arr);
		}
		return [...m.entries()].sort((a, b) => a[0] - b[0]).map(([round, calls]) => ({ round, calls }));
	});

	type Tab = 'context' | 'state' | 'tools';
	let tab = $state<Tab>('context');

	let raw = $state(false);

	let rawText = $derived.by(() =>
		flat.map((f, i) => `[${f.block.role}]\n${flatTexts[i]}`).join('\n\n')
	);

	let collapsedGroups = new SvelteSet<number>();
	let openLore = $state<string | null>(null);
	let openPool = $state<string | null>(null);
	let openCalls = new SvelteSet<string>();

	let blockEls: HTMLElement[] = [];
	let selected = $state<number | null>(null);

	$effect(() => {
		void contextInspector.message?.id;
		collapsedGroups.clear();
		openCalls.clear();
		blockEls = [];
		selected = null;
		openLore = null;
		openPool = null;
		tab = 'context';
		raw = false;
	});

	function toggleCall(key: string) {
		if (openCalls.has(key)) openCalls.delete(key);
		else openCalls.add(key);
	}

	function toggleGroup(gi: number) {
		if (collapsedGroups.has(gi)) collapsedGroups.delete(gi);
		else collapsedGroups.add(gi);
	}

	async function focusBlock(i: number) {
		const gi = flat[i]?.gi;
		if (gi !== undefined) collapsedGroups.delete(gi);
		tab = 'context';
		raw = false;
		selected = i;
		await tick();
		blockEls[i]?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') contextInspector.close();
	}
</script>

<svelte:window onkeydown={onKey} />

{#snippet entry(block: TracedBlock, fi: number, compact: boolean)}
	{@const content = resolveBlockContent(block)}
	<div
		class="entry accent-{roleClass(block.role)}"
		class:selected={selected === fi}
		bind:this={blockEls[fi]}
	>
		<div class="entry-top">
			<span class="eyebrow">
				{block.role}{#if !compact}<span class="dot">·</span>{block.origin.kind.kind.replace(
						'_',
						' '
					)}{/if}
			</span>
			<span class="num"
				>{fmtCount(display.counts[fi] ?? 0)} {t.chat.inspector.units[display.unit]}</span
			>
		</div>

		{#if !compact}
			<div class="entry-title">{block.origin.node_title || t.chat.inspector.untitledNode}</div>
			<div class="crumbs">
				{#if content.label}<span class="ref">{content.label}</span>{/if}
				{#if block.origin.kind.kind === 'source'}<span>{block.origin.kind.source_id || '—'}</span
					>{/if}
				{#if block.origin.path.length > 0}<span class="path">{block.origin.path.join(' › ')}</span
					>{/if}
				<span>{t.chat.inspector.node({ id: block.origin.node_id || '—' })}</span>
			</div>
		{/if}

		{#if block.content_ref.kind === 'template'}
			<pre
				class="content">{#each block.content_ref.parts as part, pi (pi)}{#if part.value === undefined}{part.source}{:else}<span
							class="expansion"
							{@attach tooltip(part.value || t.chat.inspector.expandsToNothing)}>{part.source}</span
						>{/if}{/each}</pre>
		{:else if content.text}
			<pre class="content">{content.text}</pre>
		{:else}
			<div class="unresolved">{t.chat.inspector.notReconstructed}</div>
		{/if}
	</div>
{/snippet}

{#if contextInspector.message}
	<div
		use:keyLayer={{ id: 'chat.contextInspector', blocking: true }}
		class="overlay"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) contextInspector.close();
		}}
	>
		<div class="modal" role="dialog" aria-modal="true" aria-label={t.chat.inspector.dialog}>
			<div class="header">
				<div class="titles">
					<h2>{t.chat.inspector.title}</h2>
					<span class="sub">
						{#if loadingContext}
							{t.common.loading}
						{:else if blocks === null}
							{t.chat.inspector.noTrace}
						{:else}
							{t.chat.inspector.blocks({ count: blocks.length })} · {fmtCount(display.total)}
							{t.chat.inspector.units[display.unit]}
							{#if templateName}
								<span class="sep">·</span>{templateName}
								{#if hintReady}
									<button
										type="button"
										class="hint-btn"
										aria-label={t.chat.inspector.generationSettings}
										{@attach tooltip({ content: setupCard, side: 'bottom' })}
									>
										<Info />
									</button>
									{#snippet setupCard()}
										<span class="hint-card">
											{#if templateDesc}
												<span class="hint-desc">{templateDesc}</span>
											{/if}
											<dl>
												<div>
													<dt>{t.chat.inspector.tokenBudget}</dt>
													<dd>
														{budgetShown === null
															? '—'
															: budgetShown === 0
																? t.chat.inspector.unlimited
																: fmtCount(budgetShown)}
													</dd>
												</div>
												{#if !profileFailed}
													{#each samplingRows as row (row.label)}
														<div>
															<dt>{row.label}</dt>
															<dd>{row.value}</dd>
														</div>
													{/each}
												{/if}
											</dl>
											{#if profileFailed}
												<span class="hint-src">{t.chat.inspector.profileFailed}</span>
											{:else if profileName}
												<span class="hint-src"
													>{t.chat.inspector.fromProfile({ name: profileName })}</span
												>
											{/if}
											{#if budgetIsLive}
												<span class="hint-src">{t.chat.inspector.budgetNotRecorded}</span>
											{/if}
										</span>
									{/snippet}
								{/if}
							{/if}
							{#if modelName}<span class="sep">·</span><span class="mono">{modelName}</span
								>{:else if profileName}<span class="sep">·</span>{profileName}{/if}
						{/if}
					</span>
				</div>
				<button
					class="icon-btn"
					{@attach tooltip(t.common.close)}
					onclick={() => contextInspector.close()}
				>
					<X />
				</button>
			</div>

			<div class="columns">
				<aside class="left">
					<div class="left-head">
						<span>{t.chat.inspector.layout}</span>
						<span class="num">{fmtCount(display.total)} {t.chat.inspector.units[display.unit]}</span
						>
					</div>
					{#if flat.length === 0}
						<div class="left-empty">{t.chat.inspector.noBlocks}</div>
					{:else}
						<div class="minimap" role="list" aria-label={t.chat.inspector.layoutAria}>
							{#each flat as f, i (i)}
								<button
									type="button"
									class="seg accent-{roleClass(f.block.role)}"
									class:active={selected === i}
									style="flex-grow: {display.counts[i] ?? 0}"
									{@attach tooltip(
										`${f.block.origin.node_title || t.chat.inspector.nodeFallback} · ${f.block.role} · ${fmtCount(
											display.counts[i] ?? 0
										)} ${t.chat.inspector.units[display.unit]}`
									)}
									aria-label={t.chat.inspector.blockAria({
										role: f.block.role,
										size: `${fmtCount(display.counts[i] ?? 0)} ${t.chat.inspector.units[display.unit]}`
									})}
									onclick={() => focusBlock(i)}
								></button>
							{/each}
						</div>
					{/if}
					<div class="legend">
						<span class="swatch accent-system"></span>{t.chat.inspector.roles.system}
						<span class="swatch accent-user"></span>{t.chat.inspector.roles.user}
						<span class="swatch accent-assistant"></span>{t.chat.inspector.roles.assistant}
					</div>

					{#if sourceRows.length > 0}
						<div class="left-head">
							<span>{t.chat.inspector.bySource}</span>
							{#if drift !== null}
								<span
									class="drift"
									{@attach tooltip(t.chat.inspector.drift({ count: fmtCount(drift) }))}
								>
									{t.chat.inspector.was({ count: fmtCount(drift) })}
								</span>
							{/if}
						</div>
						<ul class="sources">
							{#each sourceRows as [source, n] (source)}
								<li
									style="--share: {pct(n, display.total)}%"
									{@attach tooltip(
										`${SOURCE_LABELS[source]} · ${fmtCount(n)} ${t.chat.inspector.units[display.unit]} · ${pct(
											n,
											display.total
										).toFixed(1)}%`
									)}
								>
									<span class="fill" aria-hidden="true"></span>
									<span class="src-name">{SOURCE_LABELS[source]}</span>
									<span class="num">{fmtCount(n)}</span>
								</li>
							{/each}
						</ul>
					{/if}

					{#if trimmedRows.length > 0}
						<div class="left-head">
							<span>{t.chat.inspector.trimmed}</span>
						</div>
						<ul class="sources trimmed">
							{#each trimmedRows as [source, n] (source)}
								<li
									style="--share: {pct(n, trimmedMax)}%"
									{@attach tooltip(
										t.chat.inspector.shed({ source: SOURCE_LABELS[source], count: fmtCount(n) })
									)}
								>
									<span class="fill" aria-hidden="true"></span>
									<span class="src-name">{SOURCE_LABELS[source]}</span>
									<span class="num">−{fmtCount(n)}</span>
								</li>
							{/each}
						</ul>
					{/if}
				</aside>

				<section class="right">
					<div class="tabs" role="tablist">
						<button
							class="tab"
							class:active={tab === 'context'}
							role="tab"
							aria-selected={tab === 'context'}
							onclick={() => (tab = 'context')}
						>
							{t.chat.inspector.tabs.context}
							{#if groups}<span class="tab-count">{groups.length}</span>{/if}
						</button>
						<button
							class="tab"
							class:active={tab === 'state'}
							role="tab"
							aria-selected={tab === 'state'}
							onclick={() => (tab = 'state')}
						>
							{t.chat.inspector.tabs.state}
							{#if recon}<span class="tab-count"
									>{activationGroups.length + persistentPools.length}</span
								>{/if}
						</button>
						<button
							class="tab"
							class:active={tab === 'tools'}
							role="tab"
							aria-selected={tab === 'tools'}
							onclick={() => (tab = 'tools')}
						>
							{t.chat.inspector.tabs.tools}
							{#if toolCalls.length > 0}<span class="tab-count">{toolCalls.length}</span>{/if}
						</button>
						{#if tab === 'context'}
							<button
								class="raw-toggle"
								class:active={raw}
								aria-pressed={raw}
								{@attach tooltip(t.chat.inspector.rawHint)}
								onclick={() => (raw = !raw)}
							>
								{t.chat.inspector.raw}
							</button>
						{/if}
					</div>

					<div class="pane">
						{#if tab === 'context'}
							{#if loadingContext}
								<div class="empty">{t.common.loading}</div>
							{:else if groups === null}
								<div class="empty">
									{t.chat.inspector.noTraceLong}
								</div>
							{:else if groups.length === 0}
								<div class="empty">{t.chat.inspector.noContextBlocks}</div>
							{:else if raw}
								<pre class="raw">{rawText}</pre>
							{:else}
								{#each groups as group, gi (gi)}
									{#if group.blocks.length > 1}
										<div class="group">
											<button
												class="group-head"
												aria-expanded={!collapsedGroups.has(gi)}
												onclick={() => toggleGroup(gi)}
											>
												<span class="chev" class:open={!collapsedGroups.has(gi)}
													><ChevronDown /></span
												>
												<span class="group-title"
													>{group.title || t.chat.inspector.untitledNode}</span
												>
												<span class="num"
													>{fmtCount(groupTotals.get(gi) ?? 0)}
													{t.chat.inspector.units[display.unit]}</span
												>
												<span class="group-count"
													>{t.chat.inspector.msgs({ count: group.blocks.length })}</span
												>
											</button>
											{#if !collapsedGroups.has(gi)}
												{#each group.blocks as block, bi (bi)}
													{@render entry(block, groupOffsets[gi] + bi, true)}
												{/each}
											{/if}
										</div>
									{:else}
										{@render entry(group.blocks[0], groupOffsets[gi], false)}
									{/if}
								{/each}
							{/if}
						{:else if tab === 'state' && reconLoading && !recon}
							<div class="empty">{t.chat.inspector.loadingState}</div>
						{:else if tab === 'state' && !recon}
							<div class="empty">{t.chat.inspector.noState}</div>
						{:else if tab === 'state'}
							<div class="sub-label">{t.chat.inspector.activation}</div>
							{#if activationGroups.length === 0}
								<div class="unresolved">{t.chat.inspector.noActivation}</div>
							{:else}
								{#each activationGroups as lore (lore.id)}
									<button
										class="row"
										aria-expanded={openLore === lore.id}
										onclick={() => (openLore = openLore === lore.id ? null : lore.id)}
									>
										<span class="badge">T{lore.currentTurn}</span>
										{#await lorebookStore.getLorebook(lore.id)}
											<span class="row-title mono">{t.common.loading}</span>
										{:then lorebook}
											<span class="row-title mono">{lorebook?.name ?? lore.id}</span>
										{:catch}
											<span class="row-title mono">{lore.id}</span>
										{/await}
										<span class="group-count">{lore.entries.length}</span>
										<span class="chev" class:open={openLore === lore.id}><ChevronDown /></span>
									</button>
									{#if openLore === lore.id}
										<div class="detail">
											{#if lore.entries.length === 0}
												<div class="unresolved">{t.chat.inspector.noTrackedEntries}</div>
											{:else}
												{#each lore.entries as e (e.entryId)}
													<div class="kv-row">
														<span class="entry-name" {@attach tooltip(e.entryId)}>{e.entryId}</span>
														<span
															class="chip"
															class:dim={e.lastActivated === undefined}
															{@attach tooltip(
																t.chat.inspector.lastActivated({
																	turns: String(e.lastActivated ?? '—')
																})
															)}
															>{t.chat.inspector.act({
																turns: String(e.lastActivated ?? '—')
															})}</span
														>
														<span
															class="chip"
															class:dim={e.sticky === undefined}
															{@attach tooltip(
																t.chat.inspector.stickyRemaining({ turns: e.sticky ?? 0 })
															)}>{t.chat.inspector.stk({ turns: String(e.sticky ?? '—') })}</span
														>
													</div>
												{/each}
											{/if}
										</div>
									{/if}
								{/each}
							{/if}

							<div class="sub-label">{t.chat.inspector.persistent}</div>
							{#if persistentPools.length === 0}
								<div class="unresolved">{t.chat.inspector.noPersistent}</div>
							{:else}
								{#each persistentPools as pool (pool.name)}
									<button
										class="row"
										aria-expanded={openPool === pool.name}
										onclick={() => (openPool = openPool === pool.name ? null : pool.name)}
									>
										<span class="kind-tag">{t.chat.inspector.pool}</span>
										<span class="row-title">{pool.name}</span>
										<span class="group-count">{pool.entries.length}</span>
										<span class="chev" class:open={openPool === pool.name}><ChevronDown /></span>
									</button>
									{#if openPool === pool.name}
										<div class="detail">
											{#if pool.entries.length === 0}
												<div class="unresolved">{t.chat.inspector.emptyPool}</div>
											{:else}
												{#each pool.entries as kv (kv.key)}
													<div class="kv">
														<span class="kv-key" {@attach tooltip(kv.key)}>{kv.key}</span>
														<pre class="kv-val">{fmtValue(kv.value)}</pre>
													</div>
												{/each}
											{/if}
										</div>
									{/if}
								{/each}
							{/if}
						{:else}
							<div class="round-head">
								<span class="round-label">{t.chat.inspector.available}</span>
								{#if availableTools}
									<span class="group-count"
										>{t.chat.inspector.tools({ count: availableTools.length })}</span
									>
								{/if}
							</div>
							{#if availableTools === null}
								<div class="empty">{t.chat.inspector.noToolRecord}</div>
							{:else if availableTools.length === 0}
								<div class="empty">{t.chat.inspector.noToolsOffered}</div>
							{:else}
								<div class="offered">
									{#each availableTools as name (name)}
										<span class="chip">{name}</span>
									{/each}
								</div>
							{/if}

							{#if toolCalls.length === 0}
								<div class="empty">{t.chat.inspector.noToolCalls}</div>
							{/if}
							{#each toolRounds as rnd (rnd.round)}
								<div class="round-head">
									<span class="round-label">{t.chat.inspector.round({ n: rnd.round })}</span>
									<span class="group-count"
										>{t.chat.inspector.calls({ count: rnd.calls.length })}</span
									>
								</div>
								{#each rnd.calls as call, ci (ci)}
									{@const key = `${rnd.round}:${ci}`}
									<div class="tool" class:err={call.isError}>
										<button
											class="tool-head"
											aria-expanded={openCalls.has(key)}
											onclick={() => toggleCall(key)}
										>
											<span class="chev" class:open={openCalls.has(key)}><ChevronDown /></span>
											<span class="tool-name">{call.name}</span>
											<span class="chip" class:chip-err={call.isError}
												>{call.isError ? t.chat.inspector.error : t.chat.inspector.ok}</span
											>
											<span class="dur">{fmtDuration(call.durationMs)}</span>
										</button>
										{#if openCalls.has(key)}
											<div class="tool-body">
												<div class="io-label">{t.chat.inspector.input}</div>
												<pre class="io">{fmtJson(call.input)}</pre>
												<div class="io-label">{t.chat.inspector.output}</div>
												<pre class="io" class:io-err={call.isError}>{fmtJson(call.output)}</pre>
											</div>
										{/if}
									</div>
								{/each}
							{/each}
						{/if}
					</div>
				</section>
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xl);
		background: var(--color-overlay-bg);
		-webkit-backdrop-filter: var(--overlay-blur);
		backdrop-filter: var(--overlay-blur);
	}

	.modal {
		display: flex;
		flex-direction: column;
		width: min(920px, 100%);
		height: 85vh;
		background: var(--color-bg-modal);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-2xl);
		box-shadow: var(--shadow-xl);
		overflow: hidden;
	}

	.header {
		display: flex;
		flex-direction: row;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-md);
		flex: 0 0 auto;
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.titles {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.header h2 {
		margin: 0;
		color: var(--color-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h3);
	}
	.sub {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.sub .sep {
		color: var(--color-border-medium);
	}
	.sub .mono {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.columns {
		display: flex;
		flex-direction: row;
		flex: 1;
		min-height: 0;
	}

	.left {
		display: flex;
		flex-direction: column;
		flex: 0 0 200px;
		min-height: 0;
		padding: var(--spacing-md);
		gap: var(--spacing-sm);
		border-right: 1px solid var(--color-border-subtle);
	}
	.left-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.left-empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.minimap {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 2px;
		border-radius: var(--radius-sm);
		background: var(--color-bg-base);
	}
	.seg {
		flex-grow: 0;
		flex-shrink: 1;
		flex-basis: 0;
		min-height: 7px;
		width: 100%;
		padding: 0;
		border: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		opacity: 0.8;
		transition:
			opacity var(--transition-fast) var(--easing-default),
			box-shadow var(--transition-fast) var(--easing-default),
			filter var(--transition-fast) var(--easing-default);
	}
	.seg:hover {
		opacity: 1;
		filter: brightness(1.15);
	}
	.seg.active {
		opacity: 1;
		box-shadow: 0 0 0 2px var(--color-text-accent);
	}
	.seg.accent-system {
		background: var(--color-role-system);
	}
	.seg.accent-user {
		background: var(--color-role-user);
	}
	.seg.accent-assistant {
		background: var(--color-role-assistant);
	}
	.seg.accent-unknown {
		background: var(--color-border-medium);
	}

	.legend {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.sources {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.sources li {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-xs);
		height: 1.375rem;
		padding: 0 var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: var(--color-bg-base);
		overflow: hidden;
		font-size: var(--font-size-xs);
	}
	.sources .fill {
		position: absolute;
		inset: 0 auto 0 0;
		width: var(--share);
		background: var(--color-text-accent);
		opacity: 0.28;
	}
	.src-name {
		position: relative;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-secondary);
	}
	.sources .num {
		position: relative;
		flex: 0 0 auto;
		font-family: var(--font-family-mono);
	}
	.sources.trimmed .fill {
		background: var(--color-text-muted);
		opacity: 0.22;
	}
	.sources.trimmed .num {
		color: var(--color-text-muted);
	}
	.hint-btn {
		display: inline-flex;
		padding: 0;
		border: none;
		background: none;
		color: var(--color-text-muted);
		cursor: help;
		line-height: 0;
	}
	.hint-btn :global(svg) {
		width: 0.85em;
		height: 0.85em;
	}
	.hint-btn:hover,
	.hint-btn:focus-visible {
		color: var(--color-text-accent);
	}

	.hint-card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 13rem;
		padding: var(--spacing-xs) 0;
	}
	.hint-desc {
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.hint-card dl {
		display: flex;
		flex-direction: column;
		gap: 2px;
		margin: 0;
	}
	.hint-card dl div {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--spacing-md);
		font-size: var(--font-size-xs);
	}
	.hint-card dt {
		color: var(--color-text-muted);
	}
	.hint-card dd {
		margin: 0;
		font-family: var(--font-family-mono);
		color: var(--color-text-primary);
	}
	.hint-src {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.drift {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		text-transform: none;
		letter-spacing: normal;
		color: var(--color-text-muted);
		cursor: help;
	}
	.swatch {
		display: inline-block;
		width: 9px;
		height: 9px;
		border-radius: 2px;
		margin-left: var(--spacing-sm);
	}
	.legend .swatch:first-child {
		margin-left: 0;
	}
	.swatch.accent-system {
		background: var(--color-role-system);
	}
	.swatch.accent-user {
		background: var(--color-role-user);
	}
	.swatch.accent-assistant {
		background: var(--color-role-assistant);
	}

	.right {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
	.tabs {
		display: flex;
		flex: 0 0 auto;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tab {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		flex: 1;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		cursor: pointer;
		border-bottom: 2px solid transparent;
	}
	.tab.active {
		color: var(--color-text-primary);
		border-bottom-color: var(--color-text-accent);
	}
	.tab-count {
		font-family: var(--font-family-mono);
		color: var(--color-text-muted);
		letter-spacing: 0;
	}

	.raw-toggle {
		flex: 0 0 auto;
		align-self: center;
		margin-right: var(--spacing-sm);
		background-color: transparent;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		padding: 2px var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
		cursor: pointer;
	}
	.raw-toggle.active {
		background: var(--color-bg-base);
		border-color: var(--color-text-accent);
		color: var(--color-text-primary);
	}

	.pane {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-lg);
	}
	.empty {
		margin: auto;
		color: var(--color-text-muted);
		line-height: var(--line-height-normal);
		text-align: center;
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}
	.group-head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: transparent;
		border: none;
		border-bottom: 1px solid var(--color-border-subtle);
		border-radius: 0;
		padding: 0 0 var(--spacing-xs);
		text-align: left;
		cursor: pointer;
	}
	.group-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.group-count {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.entry {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) 0 var(--spacing-sm) var(--spacing-md);
		border-left: 3px solid var(--color-border-medium);
		transition: border-color var(--transition-fast) var(--easing-default);
		scroll-margin-top: var(--spacing-md);
	}
	.entry.accent-system {
		border-left-color: var(--color-role-system);
	}
	.entry.accent-user {
		border-left-color: var(--color-role-user);
	}
	.entry.accent-assistant {
		border-left-color: var(--color-role-assistant);
	}
	.entry.selected {
		border-left-width: 4px;
	}
	.entry.selected .content {
		box-shadow: inset 0 0 0 1px var(--color-text-accent);
	}

	.entry-top {
		display: flex;
		flex-direction: row;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.eyebrow {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.eyebrow .dot {
		margin: 0 var(--spacing-xs);
		color: var(--color-border-medium);
	}
	.num {
		flex: 0 0 auto;
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.entry-title {
		color: var(--color-text-primary);
		font-weight: var(--font-weight-semibold);
		line-height: var(--line-height-snug);
	}
	.crumbs {
		display: flex;
		flex-direction: row;
		flex-wrap: wrap;
		gap: var(--spacing-xs) var(--spacing-md);
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.crumbs .ref {
		color: var(--color-text-accent);
	}
	.crumbs .path {
		color: var(--color-text-secondary);
		overflow-wrap: anywhere;
	}

	.content {
		margin: var(--spacing-xs) 0 0;
		padding: var(--spacing-sm) var(--spacing-md);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.expansion {
		color: var(--color-text-accent);
		text-decoration: underline dotted;
		text-underline-offset: 3px;
		cursor: help;
	}

	.raw {
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.unresolved {
		padding: var(--spacing-sm) var(--spacing-md);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		font-style: italic;
	}

	.sub-label {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: transparent;
		border-radius: var(--radius-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		text-align: left;
		cursor: pointer;
	}
	.row:hover {
		background-color: var(--color-bg-elevated);
	}
	.badge {
		flex: 0 0 auto;
		min-width: 18px;
		padding: 0 var(--spacing-xs);
		text-align: center;
		border-radius: var(--radius-full);
		background: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.kind-tag {
		flex: 0 0 auto;
		padding: 1px var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: var(--color-info-bg);
		color: var(--color-info-text);
		border: 1px solid var(--color-info-border);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-wide);
	}
	.row-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
	}
	.row-title.mono {
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-secondary);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev.open {
		transform: rotate(180deg);
	}
	.chev :global(svg) {
		width: 16px;
		height: 16px;
	}

	.detail {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm) var(--spacing-sm);
	}
	.kv-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) 0 var(--spacing-xs) var(--spacing-md);
		border-left: 2px solid var(--color-border-subtle);
	}
	.entry-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
	}
	.chip {
		flex: 0 0 auto;
		padding: 1px var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.chip.dim {
		color: var(--color-text-muted);
		opacity: 0.6;
	}
	.kv {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding-left: var(--spacing-md);
		border-left: 2px solid var(--color-border-subtle);
	}
	.kv-key {
		color: var(--color-text-accent);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.kv-val {
		margin: 0;
		max-height: 120px;
		overflow-y: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.round-head {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		margin-top: var(--spacing-xs);
	}
	.round-label {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.offered {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-xs);
	}

	.tool {
		display: flex;
		flex-direction: column;
		border-left: 3px solid var(--color-border-medium);
		border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
	}
	.tool.err {
		border-left-color: var(--color-warning-text);
	}
	.tool-head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		background-color: transparent;
		border: none;
		border-radius: 0;
		padding: var(--spacing-xs) var(--spacing-sm);
		text-align: left;
		cursor: pointer;
	}
	.tool-head:hover {
		background-color: var(--color-bg-elevated);
	}
	.tool-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
	}
	.chip-err {
		background: var(--color-warning-bg);
		color: var(--color-warning-text);
	}
	.dur {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}

	.tool-body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm) var(--spacing-sm) var(--spacing-md);
	}
	.io-label {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.io {
		margin: 0;
		max-height: 200px;
		overflow-y: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		background: var(--color-bg-base);
		border-radius: var(--radius-sm);
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.io.io-err {
		color: var(--color-warning-text);
	}

	.icon-btn {
		flex: 0 0 auto;
		background-color: transparent;
		justify-content: center;
		align-items: center;
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.icon-btn :global(svg) {
		width: 18px;
		height: 18px;
	}
</style>
