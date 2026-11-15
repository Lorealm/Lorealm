<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { Plus, Trash2, X } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import NumberField from '#lib/components/NumberInput.svelte';
	import { showPopup } from '#lib/stores/popup.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { User } from '#lib/types/user.js';
	import {
		ADMINISTRATOR_ROLE_ID,
		CODE_EXECUTION,
		EVERYONE_ROLE_ID,
		PERMISSION_GROUPS,
		Permission,
		type Role
	} from '#lib/types/permissions.js';

	const api = useApi();

	let roles = $state<Role[]>([]);
	let loaded = $state(false);
	let selectedId = $state<Snowflake>();
	let draft = $state({ name: '', permissions: 0n, position: 0 });
	let members = $state<User[]>([]);
	let username = $state('');
	let confirmingDelete = $state(false);
	let busy = $state(false);
	let error = $state('');

	const canManage = $derived(configStore.can(Permission.MANAGE_ROLES));
	const selected = $derived(roles.find((r) => r.id === selectedId));
	const isEveryone = $derived(selected?.id === EVERYONE_ROLE_ID);
	const isAdministratorRole = $derived(selected?.id === ADMINISTRATOR_ROLE_ID);
	const editable = $derived(
		!!selected &&
			canManage &&
			(configStore.can(Permission.ADMINISTRATOR) || selected.position < configStore.highestPosition)
	);
	const membersEditable = $derived(editable && !isEveryone);

	function flagLocked(flag: bigint) {
		const grantable = configStore.can(Permission.ADMINISTRATOR) || configStore.can(flag);
		return !grantable || (isAdministratorRole && flag === Permission.ADMINISTRATOR);
	}
	const dirty = $derived(
		!!selected &&
			(draft.name !== selected.name ||
				draft.permissions !== BigInt(selected.permissions) ||
				draft.position !== selected.position)
	);

	function byPosition(a: Role, b: Role) {
		return b.position - a.position;
	}

	async function run(action: () => Promise<void>) {
		busy = true;
		error = '';
		try {
			await action();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	function select(role: Role) {
		selectedId = role.id;
		draft = { name: role.name, permissions: BigInt(role.permissions), position: role.position };
		members = [];
		username = '';
		confirmingDelete = false;
		error = '';
		if (canManage && role.id !== EVERYONE_ROLE_ID) {
			run(async () => {
				members = await api.roles.members.list(role.id);
			});
		}
	}

	function toggle(flag: bigint) {
		draft.permissions ^= flag;
	}

	function replace(role: Role) {
		roles = [...roles.filter((r) => r.id !== role.id), role].sort(byPosition);
	}

	function newRole() {
		run(async () => {
			const taken = new Set(roles.map((r) => r.name));
			let name = t.settings.roles.newName;
			for (let n = 2; taken.has(name); n++) name = t.settings.roles.newNameNumbered({ n });
			const role = await api.roles.create({ name, permissions: '0', position: 1 });
			replace(role);
			select(role);
		});
	}

	/** What each code-execution flag exposes, for the warning. */
	const codeExecutionRisks = (): Array<[bigint, string]> => [
		[Permission.BUILD_PLUGINS, t.settings.roles.buildRisk],
		[Permission.UNSAFE_PLUGINS, t.settings.roles.unsafeRisk]
	];

	/** Asks before turning on flags that let outside code run. True to go ahead. */
	async function confirmCodeExecution(enabling: bigint): Promise<boolean> {
		if (enabling === 0n) return true;
		const risks = codeExecutionRisks()
			.filter(([flag]) => (enabling & flag) === flag)
			.map(([, risk]) => risk);
		const enable = t.settings.roles.enableAnyway;
		const choice = await showPopup({
			title: t.settings.roles.confirmTitle,
			variant: 'danger',
			message: [...risks, t.settings.roles.confirmWarning, t.settings.roles.disclaimer].join(
				'\n\n'
			),
			actions: [{ label: t.common.cancel }, { label: enable, tone: 'danger' }]
		});
		return choice === enable;
	}

	async function save() {
		if (!selected) return;
		const role = selected;
		const enabling = draft.permissions & ~BigInt(role.permissions) & CODE_EXECUTION;
		if (!(await confirmCodeExecution(enabling))) return;
		run(async () => {
			const updated = await api.roles.update(role.id, {
				name: draft.name !== role.name ? draft.name : undefined,
				position: draft.position !== role.position ? draft.position : undefined,
				permissions:
					draft.permissions !== BigInt(role.permissions) ? draft.permissions.toString() : undefined
			});
			replace(updated);
			draft = {
				name: updated.name,
				permissions: BigInt(updated.permissions),
				position: updated.position
			};
		});
	}

	function reset() {
		if (selected) select(selected);
	}

	function remove() {
		if (!selected) return;
		const id = selected.id;
		run(async () => {
			await api.roles.delete(id);
			roles = roles.filter((r) => r.id !== id);
			selectedId = undefined;
		});
	}

	function addMember() {
		if (!selected || !username.trim()) return;
		const id = selected.id;
		run(async () => {
			const user = await api.roles.members.add(id, username.trim());
			if (!members.some((m) => m.id === user.id)) members = [...members, user];
			username = '';
		});
	}

	function removeMember(userId: Snowflake) {
		if (!selected) return;
		const id = selected.id;
		run(async () => {
			await api.roles.members.remove(id, userId);
			members = members.filter((m) => m.id !== userId);
		});
	}

	onMount(() => {
		run(async () => {
			roles = (await api.roles.list()).sort(byPosition);
			loaded = true;
			if (roles[0]) select(roles[0]);
		});
	});
</script>

<div class="page">
	<div class="topbar">
		<h2>{t.settings.roles.title}</h2>
		{#if canManage}
			<button
				type="button"
				class="primary"
				onclick={newRole}
				disabled={busy || configStore.highestPosition <= 1}
			>
				<Plus size={16} />
				{t.settings.roles.new}
			</button>
		{/if}
	</div>

	{#if !loaded}
		<div class="status">{error || t.settings.roles.loading}</div>
	{:else}
		<div class="layout">
			<ul class="roleList">
				{#each roles as role (role.id)}
					<li>
						<button
							type="button"
							class="roleItem"
							class:active={role.id === selectedId}
							onclick={() => select(role)}
						>
							<span class="name">{role.name}</span>
							<span class="position">{role.position}</span>
						</button>
					</li>
				{/each}
			</ul>

			{#if selected}
				<div class="editor">
					{#if !canManage}
						<p class="note">
							{t.settings.roles.viewOnly}
						</p>
					{:else if !editable}
						<p class="note">{t.settings.roles.aboveYou}</p>
					{/if}

					<fieldset disabled={!editable || busy}>
						{#if isEveryone}
							<p class="note">
								{t.settings.roles.everyone}
							</p>
						{:else}
							<div class="row">
								<label class="field grow">
									<span class="label">{t.settings.roles.name}</span>
									<input type="text" bind:value={draft.name} />
								</label>
								<div class="field">
									<span class="label">{t.settings.roles.position}</span>
									{#if isAdministratorRole}
										<span class="fixed">{selected.position}</span>
									{:else}
										<NumberField
											value={draft.position}
											min={1}
											max={configStore.highestPosition - 1}
											width="7rem"
											ariaLabel={t.settings.roles.position}
											onchange={(v) => v !== '' && (draft.position = v)}
										/>
									{/if}
								</div>
							</div>
							<p class="hint">
								{isAdministratorRole ? t.settings.roles.adminHint : t.settings.roles.positionHint}
							</p>
						{/if}

						{#each PERMISSION_GROUPS as group (group.title)}
							<h3>{group.title}</h3>
							<div class="flags">
								{#each group.items as item (item.flag)}
									<label class="check" class:locked={flagLocked(item.flag)}>
										<input
											type="checkbox"
											checked={(draft.permissions & item.flag) === item.flag}
											disabled={flagLocked(item.flag)}
											onchange={() => toggle(item.flag)}
										/>
										<span>
											<span class="flagLabel">{item.label}</span>
											<span class="hint">{item.hint}</span>
										</span>
									</label>
								{/each}
							</div>
						{/each}
					</fieldset>

					{#if editable}
						<div class="actions">
							{#if !isEveryone && selected.id !== ADMINISTRATOR_ROLE_ID}
								{#if confirmingDelete}
									<button type="button" class="danger" onclick={remove} disabled={busy}>
										{t.settings.roles.confirmDelete({ name: selected.name })}
									</button>
									<button type="button" class="ghost" onclick={() => (confirmingDelete = false)}>
										{t.settings.roles.keep}
									</button>
								{:else}
									<button
										type="button"
										class="icon"
										aria-label={t.settings.roles.delete}
										onclick={() => (confirmingDelete = true)}
									>
										<Trash2 size={16} />
									</button>
								{/if}
							{/if}
							<div class="spacer"></div>
							<button type="button" class="ghost" onclick={reset} disabled={!dirty || busy}>
								{t.settings.reset}
							</button>
							<button type="button" class="primary" onclick={save} disabled={!dirty || busy}>
								{t.common.save}
							</button>
						</div>
					{/if}

					{#if canManage && !isEveryone}
						<h3>{t.settings.roles.members}</h3>
						{#if members.length === 0}
							<p class="hint">{t.settings.roles.noMembers}</p>
						{:else}
							<ul class="members">
								{#each members as member (member.id)}
									<li>
										<span class="name">{member.display_name ?? member.username}</span>
										<span class="hint">@{member.username}</span>
										{#if membersEditable}
											<button
												type="button"
												class="icon"
												aria-label={t.settings.roles.removeMember({ name: member.username })}
												onclick={() => removeMember(member.id)}
												disabled={busy}
											>
												<X size={14} />
											</button>
										{/if}
									</li>
								{/each}
							</ul>
						{/if}
						{#if membersEditable}
							<form
								class="row"
								onsubmit={(e) => {
									e.preventDefault();
									addMember();
								}}
							>
								<input type="text" placeholder={t.settings.roles.username} bind:value={username} />
								<button type="submit" class="ghost" disabled={busy || !username.trim()}
									>{t.settings.roles.add}</button
								>
							</form>
						{/if}
					{/if}

					{#if error}<p class="error">{error}</p>{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}
	.topbar h2 {
		font-size: var(--font-size-h4);
	}
	button.primary {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}
	button.ghost {
		background: none;
	}
	button.danger {
		background: none;
		color: var(--color-error-text);
		border: 1px solid currentColor;
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		color: var(--color-text-secondary);
	}
	.icon:hover {
		color: var(--color-error-text);
		background: none;
	}

	.layout {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	.roleList {
		list-style: none;
		margin: 0;
		padding: var(--spacing-md);
		width: 14rem;
		flex-shrink: 0;
		overflow-y: auto;
		border-right: 1px solid var(--color-border-subtle);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.roleItem {
		display: flex;
		width: 100%;
		align-items: center;
		gap: var(--spacing-sm);
		background: none;
		border: 1px solid transparent;
		text-align: left;
	}
	.roleItem.active {
		border-color: var(--color-border-active);
		background-color: var(--color-bg-elevated);
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fixed {
		padding: 6px 0;
		color: var(--color-text-secondary);
	}
	.position {
		margin-left: auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.editor {
		flex: 1 1 auto;
		min-width: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}
	fieldset {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		margin: 0;
		padding: 0;
		border: none;
		min-inline-size: 0;
	}
	h3 {
		margin: var(--spacing-sm) 0 0;
	}
	.row {
		display: flex;
		align-items: flex-end;
		gap: var(--spacing-md);
	}
	.row input {
		flex: 1 1 auto;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.grow {
		flex: 1 1 auto;
	}
	.label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.flags {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		gap: var(--spacing-sm) var(--spacing-lg);
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		cursor: pointer;
	}
	.check input {
		margin-top: 3px;
	}
	.check > span {
		display: flex;
		flex-direction: column;
	}
	.check.locked {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.hint,
	.note {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.note {
		color: var(--color-warning-text);
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.spacer {
		flex: 1 1 auto;
	}
	.members {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.members li {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}
	.members .icon {
		margin-left: auto;
	}
	.status {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
	}
	.error {
		margin: 0;
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}

	@media (max-width: 640px) {
		.layout {
			flex-direction: column;
		}
		.roleList {
			width: auto;
			max-height: 12rem;
			border-right: none;
			border-bottom: 1px solid var(--color-border-subtle);
		}
	}
</style>
