<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { studioStore, type StudioResourceKind } from '$lib/stores/studio.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { ImportKind } from '$lib/types/realm.js';
	import type { Component } from 'svelte';
	import {
		BookOpen,
		Bot,
		Database,
		FileText,
		Pencil,
		Plug,
		Repeat,
		Terminal,
		User,
		UserCircle,
		Wrench,
		X
	} from '@lucide/svelte';
	import Badge from './Badge.svelte';
	import Tag from './Tag.svelte';
	import { catalogItem, realmEditor, scopeRoot, type GraphType } from './realmEditor.svelte.js';

	type Tone = 'accent' | 'info' | 'warning';
	const KIND = {
		llm: { label: 'LLM', icon: Bot, tone: 'accent' as Tone },
		player: { label: 'Player', icon: User, tone: 'info' as Tone },
		script: { label: 'Script', icon: Terminal, tone: 'warning' as Tone }
	};
	const IMPORT_ICON: Record<ImportKind, Component> = {
		character: User,
		lorebook: BookOpen,
		tool: Wrench,
		template: FileText,
		persona: UserCircle,
		connection_profile: Plug
	};
	const IMPORT_LABEL: Record<ImportKind, string> = {
		character: 'Character',
		lorebook: 'Lorebook',
		tool: 'Tool',
		template: 'Context template',
		persona: 'Persona',
		connection_profile: 'Connection profile'
	};

	interface Row {
		label: string;
		badge?: string;
		onSelect?: () => void;
		actions: Array<{ icon: Component; title: string; run: () => void }>;
	}
	interface Group {
		title: string;
		empty: string;
		rows: Row[];
	}
	interface GraphInsp {
		icon: Component;
		iconColor: string;
		label: string;
		typeLabel: string;
		primary?: { label: string; run: () => void };
		groups: Group[];
	}

	const realm = $derived(realmStore.realm);
	const tab = $derived(realmEditor.tab);
	const TYPE_COLOR: Record<GraphType, string> = {
		agent: 'var(--color-text-accent)',
		scope: 'var(--color-warning)',
		tool: 'var(--color-success)',
		lorebook: 'var(--color-info)',
		character: 'var(--color-primary-light)',
		template: 'var(--color-text-secondary)'
	};

	const resource = $derived.by(() => {
		if (tab !== 'imports' || !realmEditor.selectedImportKey) return null;
		const [kind, id] = realmEditor.selectedImportKey.split(':') as [ImportKind, Snowflake];
		const item = catalogItem(kind, id);
		if (!item) return null;
		return { kind, id, item };
	});

	const agentInsp = $derived.by(() => {
		if (tab !== 'cast' || !realmEditor.expandedAgentId) return null;
		const a = realmStore.agents.find((x) => x.id === realmEditor.expandedAgentId);
		if (!a) return null;
		const meta = KIND[a.kind];
		return {
			meta,
			name: a.name,
			sees: a.grants.length
				? a.grants.map((g) => g.scope || '(unset scope)').join(', ')
				: 'Nothing — no grants',
			does: a.tools.length
				? a.tools.map((t) => catalogItem('tool', t.tool_id)?.name ?? t.tool_id).join(', ')
				: 'Nothing — speak-only',
			knows: a.lorebooks.length
				? a.lorebooks.map((l) => catalogItem('lorebook', l)?.name ?? l).join(', ')
				: 'No lorebooks bound',
			speaks:
				a.kind === 'script'
					? "Doesn't speak — runs world logic"
					: a.speech_scope
						? 'Private → ' + a.speech_scope
						: 'Broadcast (main)'
		};
	});

	const graphInsp = $derived.by<GraphInsp | null>(() => {
		if (tab !== 'graph' || !realm || !realmEditor.graphSelectedNode) return null;
		const gsel = realmEditor.graphSelectedNode;
		const [gt, ...rest] = gsel.split(':');
		const gid = rest.join(':');
		const agents = realm.definition.agents;
		const select = (id: string) => realmEditor.selectGraphNode(id);

		if (gt === 'agent') {
			const a = agents.find((x) => x.id === gid);
			if (!a) return null;
			const meta = KIND[a.kind];
			const groups: Group[] = [];
			groups.push({
				title: 'State pools',
				empty: 'No grants',
				rows: a.grants.map((g, gi) => ({
					label: g.scope || '(unset)',
					badge: g.access === 'read_write' ? 'R/W' : 'R',
					onSelect: () => select('scope:' + g.scope),
					actions: [
						{
							icon: Repeat,
							title: 'Toggle read / read-write',
							run: () =>
								realmStore.editAgent(
									a.id,
									(x) =>
										(x.grants[gi].access = x.grants[gi].access === 'read' ? 'read_write' : 'read')
								)
						},
						{
							icon: X,
							title: 'Remove grant',
							run: () =>
								realmStore.editAgent(a.id, (x) => (x.grants = x.grants.filter((_, j) => j !== gi)))
						}
					]
				}))
			});
			groups.push({
				title: 'Tools',
				empty: 'Speak-only',
				rows: a.tools.map((tb) => ({
					label: catalogItem('tool', tb.tool_id)?.name ?? tb.tool_id,
					onSelect: () => select('tool:' + tb.tool_id),
					actions: [
						{
							icon: X,
							title: 'Unbind tool',
							run: () =>
								realmStore.editAgent(
									a.id,
									(x) => (x.tools = x.tools.filter((t) => t.tool_id !== tb.tool_id))
								)
						}
					]
				}))
			});
			groups.push({
				title: 'Lorebooks',
				empty: 'None',
				rows: a.lorebooks.map((l) => ({
					label: catalogItem('lorebook', l)?.name ?? l,
					onSelect: () => select('lorebook:' + l),
					actions: [
						{
							icon: X,
							title: 'Unbind lorebook',
							run: () =>
								realmStore.editAgent(
									a.id,
									(x) => (x.lorebooks = x.lorebooks.filter((y) => y !== l))
								)
						}
					]
				}))
			});
			const idRows: Row[] = [];
			if (a.character_id)
				idRows.push({
					label: catalogItem('character', a.character_id)?.name ?? a.character_id,
					badge: 'Character',
					onSelect: () => select('character:' + a.character_id),
					actions: [
						{
							icon: Pencil,
							title: 'Edit in Studio',
							run: () => studioStore.editResource('character', a.character_id!)
						}
					]
				});
			if (a.template_id)
				idRows.push({
					label: catalogItem('template', a.template_id)?.name ?? a.template_id,
					badge: 'Template',
					onSelect: () => select('template:' + a.template_id),
					actions: [
						{
							icon: Pencil,
							title: 'Edit in Studio',
							run: () => studioStore.editResource('template', a.template_id!)
						}
					]
				});
			if (idRows.length) groups.push({ title: 'Identity', empty: '', rows: idRows });
			return {
				icon: meta.icon,
				iconColor: 'var(--color-text-accent)',
				label: a.name,
				typeLabel: meta.label + ' agent',
				primary: {
					label: 'Open in Cast',
					run: () => {
						realmEditor.setTab('cast');
						realmEditor.expandedAgentId = a.id;
					}
				},
				groups
			};
		}

		if (gt === 'scope' || gt === 'pool') {
			const isPool = gt === 'pool';
			const matches = (scope: string) =>
				scope && (isPool ? scopeRoot(scope) === gid : scope === gid);
			const users = agents.filter((a) => a.grants.some((g) => matches(g.scope)));
			const groups: Group[] = [];
			if (isPool) {
				const scopes = [
					...new Set(
						users.flatMap((a) => a.grants.map((g) => g.scope).filter((s) => scopeRoot(s) === gid))
					)
				].sort();
				groups.push({
					title: 'Scopes',
					empty: '',
					rows: scopes.map((sc) => ({
						label: sc,
						onSelect: () => select('scope:' + sc),
						actions: []
					}))
				});
			}
			groups.push({
				title: 'Accessed by',
				empty: 'No agents access this',
				rows: users.map((a) => {
					const gi = a.grants.findIndex((g) => matches(g.scope));
					const g = a.grants[gi];
					return {
						label: a.name,
						badge: g.access === 'read_write' ? 'R/W' : 'R',
						onSelect: () => select('agent:' + a.id),
						actions: isPool
							? []
							: [
									{
										icon: Repeat,
										title: 'Toggle read / read-write',
										run: () =>
											realmStore.editAgent(
												a.id,
												(x) =>
													(x.grants[gi].access =
														x.grants[gi].access === 'read' ? 'read_write' : 'read')
											)
									},
									{
										icon: X,
										title: 'Remove grant',
										run: () =>
											realmStore.editAgent(
												a.id,
												(x) => (x.grants = x.grants.filter((_, j) => j !== gi))
											)
									}
								]
					};
				})
			});
			return {
				icon: Database,
				iconColor: 'var(--color-warning)',
				label: isPool ? gid + '.*' : gid,
				typeLabel: isPool ? 'State pool' : `State scope (${scopeRoot(gid)} pool)`,
				groups
			};
		}

		if (gt === 'tool' || gt === 'lorebook') {
			const item = catalogItem(gt, gid);
			const users =
				gt === 'tool'
					? agents.filter((a) => a.tools.some((tb) => tb.tool_id === gid))
					: agents.filter((a) => a.lorebooks.includes(gid));
			return {
				icon: gt === 'tool' ? Wrench : BookOpen,
				iconColor: TYPE_COLOR[gt],
				label: item?.name ?? gid,
				typeLabel: gt === 'tool' ? 'Tool' : 'Lorebook',
				primary: { label: 'Edit in Studio', run: () => studioStore.editResource(gt, gid) },
				groups: [
					{
						title: 'Bound to',
						empty: 'Imported, but no agent uses it',
						rows: users.map((a) => ({
							label: a.name,
							onSelect: () => select('agent:' + a.id),
							actions: [
								{
									icon: X,
									title: 'Unbind',
									run: () =>
										realmStore.editAgent(a.id, (x) =>
											gt === 'tool'
												? (x.tools = x.tools.filter((t) => t.tool_id !== gid))
												: (x.lorebooks = x.lorebooks.filter((l) => l !== gid))
										)
								}
							]
						}))
					}
				]
			};
		}

		const item = catalogItem(gt as ImportKind, gid);
		const users =
			gt === 'character'
				? agents.filter((a) => a.character_id === gid)
				: agents.filter((a) => a.template_id === gid);
		return {
			icon: gt === 'character' ? User : FileText,
			iconColor: TYPE_COLOR[gt as GraphType],
			label: item?.name ?? gid,
			typeLabel: gt === 'character' ? 'Character' : 'Context template',
			primary: {
				label: 'Edit in Studio',
				run: () => studioStore.editResource(gt as StudioResourceKind, gid)
			},
			groups: [
				{
					title: gt === 'character' ? 'Played by' : 'Used by',
					empty: 'Imported, but no agent uses it',
					rows: users.map((a) => ({
						label: a.name,
						onSelect: () => select('agent:' + a.id),
						actions: []
					}))
				}
			]
		};
	});

	const summary = $derived.by(() => {
		if (!realm) return null;
		return {
			name: realm.definition.name || 'Untitled Realm',
			description: realm.definition.description || 'No description yet.',
			agentCount: realm.definition.agents.length,
			importTotal: realm.definition.imports.length,
			tags: realm.tags
		};
	});

	const mode = $derived(
		!realm
			? 'onboarding'
			: graphInsp
				? 'graph'
				: resource
					? 'resource'
					: agentInsp
						? 'agent'
						: 'summary'
	);
</script>

{#if mode === 'onboarding'}
	<div class="pane">
		<div class="h">What's a realm?</div>
		<p>
			A reusable starting point for a world — a cast, the things they can know and do, and the rules
			binding them. Playing it starts a fresh, private conversation each time.
		</p>
	</div>
{:else if mode === 'resource' && resource}
	{@const Icon = IMPORT_ICON[resource.kind]}
	<div class="pane">
		<div class="titleRow">
			<Icon size={18} color="var(--color-text-accent)" />
			<Badge tone="accent">{IMPORT_LABEL[resource.kind]}</Badge>
		</div>
		<div class="h">{resource.item.name}</div>
		<p>{resource.item.description || 'No description.'}</p>
		{#if resource.kind !== 'connection_profile'}
			<button
				type="button"
				class="wide secondary"
				onclick={() =>
					studioStore.editResource(resource.kind as StudioResourceKind, resource.id)}
			>
				<Pencil size={14} /> Edit in Studio
			</button>
		{/if}
	</div>
{:else if mode === 'agent' && agentInsp}
	<div class="pane">
		<div class="titleRow">
			<agentInsp.meta.icon size={18} color="var(--color-text-accent)" />
			<span class="h inline">{agentInsp.name}</span>
			<Badge tone={agentInsp.meta.tone}>{agentInsp.meta.label}</Badge>
		</div>
		<div class="facts">
			<div>
				<div class="k">Sees</div>
				<div class="v">{agentInsp.sees}</div>
			</div>
			<div>
				<div class="k">Can do</div>
				<div class="v">{agentInsp.does}</div>
			</div>
			<div>
				<div class="k">Knows</div>
				<div class="v">{agentInsp.knows}</div>
			</div>
			<div>
				<div class="k">Speaks</div>
				<div class="v">{agentInsp.speaks}</div>
			</div>
		</div>
	</div>
{:else if mode === 'graph' && graphInsp}
	<div class="pane">
		<div>
			<div class="titleRow">
				<graphInsp.icon size={18} color={graphInsp.iconColor} />
				<span class="h inline break">{graphInsp.label}</span>
			</div>
			<div class="typeLabel">{graphInsp.typeLabel}</div>
		</div>
		{#if graphInsp.primary}
			<button type="button" class="wide secondary" onclick={graphInsp.primary.run}
				>{graphInsp.primary.label}</button
			>
		{/if}
		{#each graphInsp.groups as group (group.title)}
			<div class="group">
				<div class="groupTitle">{group.title}</div>
				{#each group.rows as row (row.label)}
					<div class="giRow">
						<button type="button" class="giLabel" onclick={row.onSelect}>{row.label}</button>
						{#if row.badge}<Badge tone="accent">{row.badge}</Badge>{/if}
						{#each row.actions as action (action.title)}
							<button type="button" class="giAct" title={action.title} onclick={action.run}>
								<action.icon size={13} />
							</button>
						{/each}
					</div>
				{:else}
					{#if group.empty}<div class="giEmpty">{group.empty}</div>{/if}
				{/each}
			</div>
		{/each}
	</div>
{:else if mode === 'summary' && summary}
	<div class="pane">
		<div class="h">{summary.name}</div>
		<p>{summary.description}</p>
		<div class="stats">
			<div class="stat"><span>Cast</span><span>{summary.agentCount} agents</span></div>
			<div class="stat"><span>Imports</span><span>{summary.importTotal} resources</span></div>
		</div>
		{#if summary.tags.length > 0}
			<div class="tags">
				{#each summary.tags as tag (tag)}<Tag>{tag}</Tag>{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.pane {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}

	.h {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-md);
		color: var(--color-text-primary);
	}

	.h.inline {
		font-size: var(--font-size-base);
	}

	.break {
		word-break: break-word;
	}

	p {
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
		line-height: var(--line-height-normal);
	}

	.titleRow {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}

	.typeLabel {
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-wide);
		margin-top: 2px;
	}

	.wide {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
	}

	.facts {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.k {
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-wide);
		margin-bottom: 2px;
	}

	.v {
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
		line-height: var(--line-height-snug);
	}

	.group {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.groupTitle {
		font-size: var(--font-size-xs);
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-wide);
		margin-bottom: 2px;
	}

	.giRow {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 6px;
		border-radius: var(--radius-md);
		background: var(--color-bg-elevated-2);
	}

	.giLabel {
		flex: 1;
		min-width: 0;
		background: none;
		border: none;
		text-align: left;
		font-size: var(--font-size-sm);
		color: var(--color-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.giAct {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		background: none;
		border: none;
		color: var(--color-text-muted);
	}

	.giAct:hover {
		color: var(--color-text-primary);
	}

	.giEmpty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.stats {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.stat {
		display: flex;
		justify-content: space-between;
		font-size: var(--font-size-base);
	}

	.stat span:first-child {
		color: var(--color-text-secondary);
	}

	.stat span:last-child {
		color: var(--color-text-primary);
	}

	.tags {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}
</style>
