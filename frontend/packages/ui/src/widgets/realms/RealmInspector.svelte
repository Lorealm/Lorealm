<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { ExternalLink, Maximize2, Plus, Trash2 } from '@lucide/svelte';
	import ContextField from '#lib/components/ContextField.svelte';
	import NumberInput from '#lib/components/NumberInput.svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { StudioResource } from '#lib/navigation/navigator.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { AgentKind, RealmResourceKind } from '#lib/types/realm.js';
	import ObjectFields from './ObjectFields.svelte';
	import RadioField from './RadioField.svelte';
	import { realmDraft } from './draft.svelte.js';
	import { realmVariables } from './variables.js';
	import { panes } from './panes.svelte.js';
	import { activeDrag, clearDrag, readDrag } from './drag.svelte.js';
	import {
		baseOf,
		declarationLine,
		declaredText,
		TYPE_NAME,
		holdsToBase,
		holdsValue,
		isList,
		loopReaches,
		referencePaths,
		isRequired,
		withList,
		type AgentBlock,
		type Binding,
		type InputDecl,
		type InputField,
		type InputType,
		type Property
	} from './model.js';
	import { RESOURCE_KINDS } from './resources.svelte.js';

	const STUDIO_KINDS: Partial<Record<RealmResourceKind, StudioResource>> = {
		character: 'character',
		lorebook: 'lorebook',
		template: 'template',
		tool: 'tool',
		persona: 'persona',
		widget: 'widget',
		module: 'module'
	};

	const AGENT_KINDS: AgentKind[] = ['llm', 'player', 'script'];

	const selection = $derived(realmDraft.selection);
	const agent = $derived(selection?.kind === 'agent' ? realmDraft.agent(selection.key) : null);
	const loop = $derived(
		selection?.kind === 'foreach'
			? realmDraft.cast.find((e) => e.key === selection.key && e.kind === 'foreach')
			: undefined
	);
	const input = $derived(
		selection?.kind === 'input'
			? realmDraft.inputs.find((i) => i.name === selection.name)
			: undefined
	);
	const resource = $derived(selection?.kind === 'resource' ? selection : undefined);

	const scope = $derived(agent ? realmDraft.scopeOf(agent.key) : []);
	const inputs = $derived(realmDraft.inputs);
	/** How the enclosing loop's entity is reached */
	const loopReach = $derived.by(() => {
		const looped = scope[0];
		if (!looped?.input) return null;
		const reach = loopReaches(looped.input.ty)[0];
		return reach ? [looped.varName, ...reach.path].join('.') : null;
	});
	const incoming = $derived(activeDrag());

	function interpolation(name: string, field: 'slug' | 'name'): string | null {
		const looped = scope.find((v) => v.input?.name === name);
		if (looped?.input) {
			const reach = loopReaches(looped.input.ty)[0];
			return reach ? `{{${[looped.varName, ...reach.path, field].join('.')}}}` : null;
		}
		const decl = inputs.find((i) => i.name === name);
		if (!decl || isList(decl.ty)) return null;
		const reach = referencePaths(decl.ty)[0];
		return reach ? `{{input:${[name, ...reach.path, field].join('.')}}}` : `{{input:${name}}}`;
	}

	function textTakes(field: 'slug' | 'name'): boolean {
		return incoming?.drag === 'input' && interpolation(incoming.name, field) !== null;
	}

	/** Splice the interpolation in at the caret rather than replacing the field. */
	function dropIntoText(event: DragEvent, field: 'slug' | 'name', apply: (next: string) => void) {
		const payload = readDrag(event);
		clearDrag();
		if (payload?.drag !== 'input') return;
		const snippet = interpolation(payload.name, field);
		if (!snippet) return;
		event.preventDefault();
		const el = event.currentTarget as HTMLInputElement;
		const from = el.selectionStart ?? el.value.length;
		const to = el.selectionEnd ?? from;
		apply(el.value.slice(0, from) + snippet + el.value.slice(to));
	}

	/** Leave the editor for wherever a resource is really edited. */
	function openResource(kind: RealmResourceKind, id: Snowflake) {
		if (kind === 'connection_profile') {
			navigator.start().connections(id);
			return;
		}
		const studio = STUDIO_KINDS[kind];
		if (studio) navigator.start().studio().resource(studio, id);
	}

	function locate(binding: Binding, mode: 'select' | 'open') {
		if (binding.source === 'input') {
			realmDraft.select({ kind: 'input', name: binding.name });
			return;
		}
		if (binding.source === 'loop') {
			const found = scope.find((v) => v.varName === binding.varName);
			if (found?.input) realmDraft.select({ kind: 'input', name: found.input.name });
			return;
		}
		if (mode === 'open') {
			openResource(binding.kind, binding.id);
			return;
		}
		realmDraft.select({
			kind: 'resource',
			resourceKind: binding.kind,
			id: binding.id,
			name: binding.name
		});
	}

	function shows(a: AgentBlock, field: 'character' | 'template' | 'persona' | 'script') {
		if (field === 'script') return a.kind === 'script';
		if (field === 'character') return a.kind === 'llm';
		if (field === 'persona') return a.kind === 'player';
		return a.kind !== 'script';
	}

	function retypeBounds(decl: InputDecl, many: boolean, min: number, max: number) {
		const base = baseOf(decl.ty);
		if (base.type !== 'range' && base.type !== 'span') return;
		const low = Math.min(min, max);
		const high = Math.max(min, max);
		realmDraft.retypeInput(decl.name, withList({ type: base.type, min: low, max: high }, many));
	}

	function retypeFields(decl: InputDecl, many: boolean, fields: InputField[]) {
		realmDraft.retypeInput(decl.name, withList({ type: 'object', fields }, many));
	}

	function retype(decl: InputDecl, base: InputType, many: boolean) {
		const previous = baseOf(decl.ty);
		realmDraft.retypeInput(decl.name, withList(base, many));
		if (many || base.type === 'reference' || base.type !== previous.type) {
			decl.defaultValue = null;
		}
	}

	const noun = (kind: RealmResourceKind) => TYPE_NAME[kind].toLocaleLowerCase();

	const blankProperty = (): Property => ({ key: '', value: '' });

	function addGrant(agent: AgentBlock) {
		agent.grants.push({ namespace: '', scope: '*', access: 'read' });
	}

	function editProperty(agent: AgentBlock, index: number, patch: Partial<Property>) {
		if (index === agent.properties.length) {
			agent.properties.push({ ...blankProperty(), ...patch });
			return;
		}
		agent.properties[index] = { ...agent.properties[index], ...patch };
	}

	const PREVIEW_LINES = 4;

	function preview(source: string): { text: string; truncated: boolean } {
		const lines = source.split('\n');
		while (lines.length > 0 && !lines[0].trim()) lines.shift();
		return {
			text: lines.slice(0, PREVIEW_LINES).join('\n'),
			truncated: lines.length > PREVIEW_LINES
		};
	}
</script>

{#snippet boundsEditor(min: number, max: number, apply: (min: number, max: number) => void)}
	<div class="bounds">
		<label>
			<span>{t.realms.inspector.from}</span>
			<NumberInput
				value={min}
				ariaLabel={t.realms.inspector.lowest}
				onchange={(next) => apply(Number(next) || 0, max)}
			/>
		</label>
		<label>
			<span>{t.realms.inspector.to}</span>
			<NumberInput
				value={max}
				ariaLabel={t.realms.inspector.highest}
				onchange={(next) => apply(min, Number(next) || 0)}
			/>
		</label>
	</div>
{/snippet}

<div class="inspector">
	{#if agent}
		{@const a = agent}
		<h3>{t.realms.inspector.agent}</h3>
		<label>
			<span>{t.realms.inspector.handle}</span>
			<input
				class="mono"
				class:candidate={textTakes('slug')}
				class:incompatible={incoming !== null && !textTakes('slug')}
				value={a.handle}
				spellcheck="false"
				oninput={(e) => (a.handle = e.currentTarget.value)}
				ondragover={(e) => textTakes('slug') && e.preventDefault()}
				ondrop={(e) => dropIntoText(e, 'slug', (next) => (a.handle = next))}
			/>
		</label>
		<p class="note">
			{t.realms.inspector.handleNote}
			<br />
			<br />
			{#if loopReach}
				<span class="warn"
					>{t.realms.inspector.loopWarning} <code>&lbrace;&lbrace;{loopReach}&rbrace;&rbrace;</code>
					<br />
					<br />
					{t.realms.inspector.forExample}
					<code>regent_&lbrace;&lbrace;{loopReach}.slug&rbrace;&rbrace;</code>.
				</span>
			{/if}
		</p>

		<label>
			<span>{t.realms.inspector.name}</span>
			<input
				class:candidate={textTakes('name')}
				class:incompatible={incoming !== null && !textTakes('name')}
				value={a.name}
				oninput={(e) => (a.name = e.currentTarget.value)}
				ondragover={(e) => textTakes('name') && e.preventDefault()}
				ondrop={(e) => dropIntoText(e, 'name', (next) => (a.name = next))}
			/>
		</label>

		<label>
			<span>{t.realms.inspector.kind}</span>
			<select value={a.kind} onchange={(e) => (a.kind = e.currentTarget.value as AgentKind)}>
				{#each AGENT_KINDS as kind (kind)}
					<option value={kind}>{kind}</option>
				{/each}
			</select>
		</label>

		{#if shows(a, 'character')}
			<div class="field">
				<span>{t.realms.inspector.character}</span>
				<RadioField
					value={a.character}
					kinds={['character']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.character}
					onlocate={locate}
					onchange={(next) => (a.character = next)}
				/>
			</div>
		{/if}

		{#if shows(a, 'template')}
			<div class="field">
				<span>{t.realms.inspector.contextTemplate}</span>
				<RadioField
					value={a.template}
					kinds={['template']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.contextTemplate}
					onlocate={locate}
					onchange={(next) => (a.template = next)}
				/>
			</div>
			<div class="field">
				<span>{t.realms.inspector.connection}</span>
				<RadioField
					value={a.profile}
					kinds={['connection_profile']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.connectionProfile}
					onlocate={locate}
					onchange={(next) => (a.profile = next)}
				/>
			</div>
		{/if}

		{#if shows(a, 'persona')}
			<div class="field">
				<span>{t.realms.inspector.persona}</span>
				<RadioField
					value={a.persona}
					kinds={['persona']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.persona}
					onlocate={locate}
					onchange={(next) => (a.persona = next)}
				/>
			</div>
		{/if}

		<div class="field">
			<span>{t.realms.inspector.lorebooks}</span>
			{#each a.lorebooks as book, i (i)}
				<RadioField
					value={book}
					kinds={['lorebook']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.lorebook({ n: i + 1 })}
					onlocate={locate}
					onchange={(next) => {
						if (next) a.lorebooks[i] = next;
						else a.lorebooks.splice(i, 1);
					}}
				/>
			{/each}
			<RadioField
				value={null}
				kinds={['lorebook']}
				{scope}
				{inputs}
				placeholder={t.realms.inspector.bindLorebook}
				ariaLabel={t.realms.inspector.bindAnotherLorebook}
				onchange={(next) => next && a.lorebooks.push(next)}
			/>
		</div>

		<div class="field">
			<span>{t.realms.inspector.widgets}</span>
			{#each a.widgets as widget, i (i)}
				<RadioField
					value={widget}
					kinds={['widget']}
					{scope}
					{inputs}
					ariaLabel={t.realms.inspector.widget({ n: i + 1 })}
					onlocate={locate}
					onchange={(next) => {
						if (next) a.widgets[i] = next;
						else a.widgets.splice(i, 1);
					}}
				/>
			{/each}
			<RadioField
				value={null}
				kinds={['widget']}
				{scope}
				{inputs}
				placeholder={t.realms.inspector.bindWidget}
				ariaLabel={t.realms.inspector.bindAnotherWidget}
				onchange={(next) => next && a.widgets.push(next)}
			/>
		</div>

		<div class="field">
			<span>{t.realms.inspector.tools}</span>
			{#each a.tools as tool, i (i)}
				<div class="stack">
					<RadioField
						value={tool.tool}
						kinds={['tool']}
						{scope}
						{inputs}
						ariaLabel={t.realms.inspector.tool({ n: i + 1 })}
						onlocate={locate}
						onchange={(next) => {
							if (next) a.tools[i].tool = next;
							else a.tools.splice(i, 1);
						}}
					/>
					<label class="inline">
						<span>{t.realms.inspector.announcesOn}</span>
						<input
							class="mono channel"
							value={tool.eventChannel}
							placeholder={t.realms.inspector.nobody}
							spellcheck="false"
							oninput={(e) => (a.tools[i].eventChannel = e.currentTarget.value)}
						/>
					</label>
				</div>
			{/each}
			<RadioField
				value={null}
				kinds={['tool']}
				{scope}
				{inputs}
				placeholder={t.realms.inspector.bindTool}
				ariaLabel={t.realms.inspector.bindAnotherTool}
				onchange={(next) => next && a.tools.push({ tool: next, eventChannel: '' })}
			/>
		</div>

		<div class="field">
			<span class="withAction">
				{t.realms.inspector.grants}
				<button type="button" aria-label={t.realms.inspector.addGrant} onclick={() => addGrant(a)}>
					<Plus size={12} />
				</button>
			</span>
			{#if a.grants.length > 0}
				<table>
					<thead>
						<tr>
							<th>{t.realms.inspector.namespace}</th>
							<th>{t.realms.inspector.scope}</th>
							<th class="narrow">{t.realms.inspector.access}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each a.grants as grant, i (i)}
							<tr>
								<td>
									<input
										class="mono"
										value={grant.namespace}
										placeholder="world"
										spellcheck="false"
										aria-label={t.realms.inspector.grantNamespace({ n: i + 1 })}
										oninput={(e) => (a.grants[i].namespace = e.currentTarget.value)}
									/>
								</td>
								<td>
									<input
										class="mono"
										value={grant.scope}
										placeholder="*"
										spellcheck="false"
										aria-label={t.realms.inspector.grantScope({ n: i + 1 })}
										oninput={(e) => (a.grants[i].scope = e.currentTarget.value)}
									/>
								</td>
								<td>
									<button
										type="button"
										class="access"
										class:write={grant.access === 'read_write'}
										{@attach tooltip(t.realms.inspector.changeAccess)}
										onclick={() =>
											(a.grants[i].access = grant.access === 'read' ? 'read_write' : 'read')}
									>
										{grant.access === 'read'
											? t.realms.inspector.read
											: t.realms.inspector.readWrite}
									</button>
								</td>
								<td>
									<button
										type="button"
										class="remove"
										aria-label={t.realms.inspector.removeGrant({ n: i + 1 })}
										onclick={() => a.grants.splice(i, 1)}
									>
										<Trash2 size={12} />
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{:else}
				<p class="note">{t.realms.inspector.ownOnly} <code>agents:{a.handle || '…'}</code>.</p>
			{/if}
		</div>

		<div class="field">
			<span>{t.realms.inspector.properties}</span>
			<table>
				<thead>
					<tr>
						<th>{t.realms.inspector.key}</th>
						<th>{t.realms.inspector.value}</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each [...a.properties, blankProperty()] as property, i (i)}
						{@const trailing = i === a.properties.length}
						<tr class:trailing>
							<td>
								<input
									class="mono"
									value={property.key}
									placeholder={t.realms.inspector.keyPlaceholder}
									spellcheck="false"
									aria-label={t.realms.inspector.propertyKey({ n: i + 1 })}
									oninput={(e) => editProperty(a, i, { key: e.currentTarget.value })}
								/>
							</td>
							<td>
								<input
									class="mono"
									value={property.value}
									placeholder={t.realms.inspector.valuePlaceholder}
									spellcheck="false"
									aria-label={t.realms.inspector.propertyValue({ n: i + 1 })}
									oninput={(e) => editProperty(a, i, { value: e.currentTarget.value })}
								/>
							</td>
							<td>
								{#if !trailing}
									<button
										type="button"
										class="remove"
										aria-label={t.realms.inspector.removeProperty({ n: i + 1 })}
										onclick={() => a.properties.splice(i, 1)}
									>
										<Trash2 size={12} />
									</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<p class="note">
				{t.realms.inspector.accessibleAs} <code>agents:{a.handle || '…'}.&lt;key&gt;</code>.
			</p>
		</div>

		<ContextField
			label={t.realms.inspector.runCondition}
			value={a.run_condition}
			dialect="expression"
			profile="realm_condition"
			variables={realmVariables(a.key)}
			multiline={false}
			tokens={false}
			maximizable={false}
			hint={t.realms.inspector.runConditionHint}
			onchange={(next) => (a.run_condition = next)}
		/>

		{#if shows(a, 'script')}
			{@const shown = preview(a.script)}
			<div class="field">
				<span>{t.realms.inspector.script}</span>
				<button
					type="button"
					class="scriptCard"
					class:blank={!shown.text}
					class:truncated={shown.truncated}
					aria-label={shown.text
						? t.realms.inspector.openScript({ name: a.handle || t.realms.inspector.thisAgent })
						: t.realms.inspector.writeScript({ name: a.handle || t.realms.inspector.thisAgent })}
					onclick={() => panes.show({ kind: 'script', agentKey: a.key })}
				>
					{#if shown.text}
						<pre aria-hidden="true">{shown.text}</pre>
					{:else}
						<span class="blankText">{t.realms.overview.nothingWritten}</span>
					{/if}
					<span class="openOverlay" aria-hidden="true">
						<Maximize2 size={12} />
						{shown.text ? t.realms.overview.open : t.realms.overview.writeOne}
					</span>
				</button>
				<p class="note">{t.realms.inspector.scriptNote}</p>
			</div>
		{/if}
	{:else if loop?.kind === 'foreach'}
		{@const l = loop}
		{@const decl = realmDraft.inputs.find((i) => i.name === l.over)}
		<h3>{t.realms.inspector.forEach}</h3>
		<label>
			<span>{t.realms.inspector.variable}</span>
			<input
				class="mono"
				value={l.varName}
				spellcheck="false"
				oninput={(e) => (l.varName = e.currentTarget.value)}
			/>
		</label>
		<label>
			<span>{t.realms.inspector.over}</span>
			<select
				value={l.over ?? ''}
				onchange={(e) => {
					l.over = e.currentTarget.value || null;
					realmDraft.clearStaleBindings(l.key);
				}}
			>
				<option value="">{t.realms.castPane.pickInput}</option>
				{#each realmDraft.inputs.filter((i) => i.ty.type === 'list') as i (i.name)}
					<option value={i.name}>{i.name} · {declaredText(i.ty)}</option>
				{/each}
			</select>
		</label>
		{#if !decl}
			<p class="note">{t.realms.inspector.pickList}</p>
		{/if}
	{:else if input}
		{@const decl = input}
		{@const base = baseOf(decl.ty)}
		{@const many = isList(decl.ty)}
		<h3>{t.realms.inspector.input}</h3>
		<p class="note">{t.realms.inspector.inputNote}</p>

		<label>
			<span>{t.realms.inspector.name}</span>
			<input
				class="mono"
				value={decl.name}
				spellcheck="false"
				onchange={(e) => realmDraft.renameInput(decl.name, e.currentTarget.value)}
			/>
		</label>

		<label>
			<span>{t.realms.inspector.description}</span>
			<textarea
				rows="3"
				value={decl.doc}
				placeholder={t.realms.inspector.descriptionPlaceholder}
				oninput={(e) => (decl.doc = e.currentTarget.value)}
			></textarea>
		</label>

		<label>
			<span>{t.realms.inspector.holds}</span>
			<select
				value={holdsValue(decl.ty)}
				onchange={(e) => retype(decl, holdsToBase(e.currentTarget.value), many)}
			>
				<optgroup label={t.realms.inspector.values}>
					<option value="string">{t.realms.fields.types.string}</option>
					<option value="number">{t.realms.fields.types.number}</option>
					<option value="bool">{t.realms.fields.types.bool}</option>
					<option value="enum">{t.realms.fields.types.enum}</option>
					<option value="range">{t.realms.fields.types.range}</option>
					<option value="span">{t.realms.fields.types.span}</option>
					<option value="object">{t.realms.fields.types.object}</option>
				</optgroup>
				<optgroup label={t.realms.inspector.owned}>
					{#each RESOURCE_KINDS as kind (kind)}
						<option value="ref:{kind}">{TYPE_NAME[kind]}</option>
					{/each}
				</optgroup>
			</select>
		</label>

		<label class="check">
			<input
				type="checkbox"
				checked={many}
				onchange={(e) => retype(decl, base, e.currentTarget.checked)}
			/>
			<span>{t.realms.inspector.isList}</span>
		</label>
		<p class="note">
			{t.realms.inspector.listNotePrefix} <code>for each</code>
			{t.realms.inspector.listNoteSuffix}
		</p>

		{#if base.type === 'range' || base.type === 'span'}
			{@const bounds = base}
			{@render boundsEditor(bounds.min, bounds.max, (min, max) =>
				retypeBounds(decl, many, min, max)
			)}
			<p class="note">
				{base.type === 'span' ? t.realms.inspector.spanNote : t.realms.inspector.rangeNote}
			</p>
		{/if}
		{#if base.type === 'object'}
			{@const group = base}
			<ObjectFields
				fields={group.fields}
				ping={realmDraft.fieldPing?.name === decl.name ? realmDraft.fieldPing : null}
				onchange={(next) => retypeFields(decl, many, next)}
			/>
		{/if}

		{#if base.type === 'enum'}
			<label>
				<span>{t.realms.inspector.choices}</span>
				<input
					value={base.variants.join(', ')}
					placeholder={t.realms.inspector.choicesPlaceholder}
					onchange={(e) => {
						const variants = e.currentTarget.value
							.split(',')
							.map((v) => v.trim())
							.filter(Boolean);
						realmDraft.retypeInput(decl.name, withList({ type: 'enum', variants }, many));
						if (decl.defaultValue !== null && !variants.includes(decl.defaultValue)) {
							decl.defaultValue = null;
						}
					}}
				/>
			</label>
		{/if}

		{#if base.type === 'reference'}
			<p class="note">
				{t.realms.inspector.referenceNote({ noun: noun(base.kind ?? 'character') })}
			</p>
		{:else if many}
			<p class="note">{t.realms.inspector.listNoDefault}</p>
		{:else}
			<label>
				<span>{t.realms.inspector.default}</span>
				{#if base.type === 'enum'}
					<select
						value={decl.defaultValue ?? ''}
						onchange={(e) => (decl.defaultValue = e.currentTarget.value || null)}
					>
						<option value="">{t.realms.inspector.noDefault}</option>
						{#each base.variants as variant (variant)}
							<option value={variant}>{variant}</option>
						{/each}
					</select>
				{:else if base.type === 'bool'}
					<select
						value={decl.defaultValue ?? ''}
						onchange={(e) => (decl.defaultValue = e.currentTarget.value || null)}
					>
						<option value="">{t.realms.inspector.noDefault}</option>
						<option value="true">{t.realms.inspector.yes}</option>
						<option value="false">{t.realms.inspector.no}</option>
					</select>
				{:else if base.type === 'number' || base.type === 'range'}
					<NumberInput
						value={decl.defaultValue === null ? '' : Number(decl.defaultValue)}
						min={base.type === 'range' ? base.min : undefined}
						max={base.type === 'range' ? base.max : undefined}
						placeholder={t.realms.inspector.noDefault}
						ariaLabel={t.realms.inspector.default}
						onchange={(next) => (decl.defaultValue = next === '' ? null : String(next))}
					/>
				{:else}
					<input
						value={decl.defaultValue ?? ''}
						placeholder={t.realms.inspector.noDefault}
						onchange={(e) => (decl.defaultValue = e.currentTarget.value.trim() || null)}
					/>
				{/if}
			</label>
		{/if}

		<div class="declaration">
			<span class="declLabel">{t.realms.inspector.declares}</span>
			<code>{declarationLine(decl)}</code>
			<span class="declTag" class:optional={!isRequired(decl)}>
				{isRequired(decl) ? t.realms.inspector.required : t.realms.inspector.optional}
			</span>
		</div>

		<button type="button" class="danger" onclick={() => realmDraft.removeInput(decl.name)}>
			<Trash2 size={12} />
			{t.realms.inspector.deleteInput}
		</button>
	{:else if resource}
		<h3>{TYPE_NAME[resource.resourceKind]}</h3>
		<p class="resourceName">{resource.name}</p>
		<p class="note"><code>{resource.id}</code></p>
		<button type="button" onclick={() => openResource(resource.resourceKind, resource.id)}>
			<ExternalLink size={12} />
			{t.realms.inspector.openInStudio}
		</button>
	{:else}
		<p class="nothing">{t.realms.inspector.nothing}</p>
	{/if}
</div>

<style>
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-md);
		min-width: 0;
	}
	h3 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
		font-weight: var(--font-weight-semibold);
		color: var(--color-text-primary);
	}
	label,
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	label > span,
	.field > span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}
	.mono {
		font-family: var(--font-family-mono);
	}

	input:not([type='checkbox'], .channel),
	select,
	textarea,
	.inspector :global(.number-field input),
	.inspector :global(.radio:not(.dropping) .field) {
		background-color: var(--color-bg-base);
		border: 1px solid var(--color-border-subtle);
	}
	input:not([type='checkbox'], .channel):focus,
	select:focus,
	textarea:focus,
	.inspector :global(.number-field input:focus),
	.inspector :global(.radio .field:is(:hover, :focus-within)) {
		background-color: var(--color-bg-base);
		border-color: var(--color-border-active);
	}

	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
	}
	.note code {
		font-family: var(--font-family-mono);
		color: var(--color-text-secondary);
	}
	.danger {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background: transparent;
		border: 1px solid var(--color-error-border);
		border-radius: var(--radius-sm);
		color: var(--color-error-text);
		font-size: var(--font-size-xs);
	}
	.danger:hover {
		background-color: var(--color-error-bg);
	}
	.candidate {
		border-color: var(--color-border-active) !important;
		border-style: dashed !important;
	}
	.incompatible {
		opacity: 0.35;
		filter: grayscale(1);
	}

	.check {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.check input {
		flex: 0 0 auto;
		margin: 0;
	}
	.check > span {
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
	}

	.declaration {
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		gap: var(--spacing-xs) var(--spacing-sm);
		padding: var(--spacing-sm);
		background-color: var(--color-bg-base);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
	}
	.declLabel {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	.declaration code {
		flex: 1 1 100%;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		color: var(--color-text-accent);
		overflow-wrap: anywhere;
	}
	.declTag {
		padding: 1px 6px;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.declTag.optional {
		color: var(--color-text-muted);
	}

	.scriptCard {
		position: relative;
		display: block;
		width: 100%;
		min-width: 0;
		padding: var(--spacing-sm);
		background-color: var(--color-bg-base);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		text-align: left;
		overflow: hidden;
	}
	.scriptCard:hover,
	.scriptCard:focus-visible {
		border-color: var(--color-border-active);
	}

	.scriptCard pre {
		margin: 0;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		line-height: var(--line-height-snug);
		color: var(--color-text-muted);
		white-space: pre;
		mask-image: linear-gradient(to right, #000 75%, transparent);
	}
	.scriptCard.truncated pre {
		mask-image:
			linear-gradient(to right, #000 75%, transparent),
			linear-gradient(to bottom, #000 55%, transparent);
		-webkit-mask-composite: source-in;
		mask-composite: intersect;
	}

	.scriptCard.blank {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 3.5em;
		border-style: dashed;
	}
	.blankText {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		font-style: italic;
	}

	.openOverlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		background-color: var(--color-bg-overlay);
		-webkit-backdrop-filter: blur(2px);
		backdrop-filter: blur(2px);
		color: var(--color-text-accent);
		font-size: var(--font-size-xs);
		opacity: 0;
		transition: opacity 120ms ease;
	}
	.scriptCard:hover .openOverlay,
	.scriptCard:focus-visible .openOverlay {
		opacity: 1;
	}
	table,
	.inspector :global(table) {
		background-color: var(--color-bg-base);
	}
	table {
		width: 100%;
		border-collapse: collapse;
		table-layout: fixed;
	}
	th {
		padding: 2px var(--spacing-xs);
		text-align: left;
		font-weight: var(--font-weight-normal);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	th:last-child {
		width: 24px;
	}
	th.narrow {
		width: 5.5em;
	}
	td {
		padding: 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	td :global(input:not([type='checkbox'])),
	td :global(select) {
		width: 100%;
		min-width: 0;
		padding: 3px var(--spacing-xs);
		background-color: transparent;
		border: none;
		border-radius: 0;
		font-size: var(--font-size-sm);
	}
	td :global(input:not([type='checkbox']):focus),
	td :global(select:focus) {
		background-color: var(--color-bg-elevated-2);
	}
	tbody tr:hover td {
		background-color: var(--color-hover);
	}
	tr.trailing :global(input),
	tr.trailing :global(select) {
		color: var(--color-text-muted);
	}
	.access {
		width: 100%;
		padding: 2px 6px;
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-align: left;
	}
	.access.write {
		color: var(--color-warning-text);
	}
	.access:hover {
		color: var(--color-text-accent);
	}

	.remove {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		padding: var(--spacing-xs) 0;
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.remove:hover {
		color: var(--color-error-text);
	}

	.withAction {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.withAction button {
		display: flex;
		padding: 2px;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
	}
	.withAction button:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.inline {
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
		padding-left: var(--spacing-sm);
	}
	.inline > span {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.channel {
		flex: 1 1 auto;
		min-width: 0;
		padding: 2px var(--spacing-xs);
		background-color: transparent;
		border-bottom: 1px solid var(--color-border-subtle);
		border-radius: 0;
		font-size: var(--font-size-xs);
	}
	.channel:focus {
		border-bottom-color: var(--color-border-active);
	}

	.resourceName {
		margin: 0;
		color: var(--color-text-primary);
		font-size: var(--font-size-md);
	}
	.inspector > button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.inspector > button:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.nothing {
		margin: var(--spacing-lg) 0;
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.warn {
		color: var(--color-warning-text);
	}
</style>
