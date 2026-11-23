<script lang="ts">
	import ConfirmDelete from '#lib/components/ConfirmDelete.svelte';
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { Clock, Copy, LogOut, Pencil, Plus, Shield, Trash2, Users, X } from '@lucide/svelte';
	import Badge from '#lib/widgets/realms/Badge.svelte';
	import { tooltip } from '#lib/stores/tooltip.svelte.js';
	import { useApi } from '#lib/api/client.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import NumberField from '#lib/components/NumberInput.svelte';
	import type { Snowflake } from '#lib/types/core.js';
	import type { AdminUser, Invite } from '#lib/types/user.js';
	import { EVERYONE_ROLE_ID, Permission, type Role } from '#lib/types/permissions.js';
	import { showToast } from '#lib/stores/errorToast.svelte.js';

	const api = useApi();

	let users = $state<AdminUser[]>([]);
	let invites = $state<Invite[]>([]);
	let roles = $state<Role[]>([]);
	let loaded = $state(false);
	let busy = $state(false);
	let error = $state('');

	let creating = $state(false);
	let newUser = $state({ username: '', email: '', password: '' });

	let editingId = $state<Snowflake>();
	let edit = $state({ display_name: '', email: '', password: '' });
	let confirmingDelete = $state<Snowflake>();

	let newInvite = $state<{ max_uses: number | ''; days: number | ''; role_id: Snowflake | '' }>({
		max_uses: 1,
		days: 7,
		role_id: ''
	});
	let createdCode = $state('');

	const canManage = $derived(configStore.can(Permission.MANAGE_USERS));
	const assignableRoles = $derived(
		configStore.can(Permission.MANAGE_ROLES)
			? roles.filter(
					(r) =>
						r.id !== EVERYONE_ROLE_ID &&
						(configStore.can(Permission.ADMINISTRATOR) || r.position < configStore.highestPosition)
				)
			: []
	);

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

	function roleName(id: Snowflake | null) {
		return roles.find((r) => r.id === id)?.name ?? t.settings.accounts.unknownRole;
	}

	function date(iso: string) {
		return new Date(iso).toLocaleDateString();
	}

	function replace(user: AdminUser) {
		users = users.map((u) => (u.id === user.id ? user : u));
	}

	function createUser() {
		run(async () => {
			const user = await api.users.admin.create({
				username: newUser.username.trim(),
				email: newUser.email.trim() || undefined,
				password: newUser.password
			});
			users = [...users, user];
			newUser = { username: '', email: '', password: '' };
			creating = false;
		});
	}

	function startEdit(user: AdminUser) {
		editingId = user.id;
		confirmingDelete = undefined;
		edit = { display_name: user.display_name ?? '', email: user.email ?? '', password: '' };
	}

	function saveEdit(user: AdminUser) {
		run(async () => {
			const updated = await api.users.admin.update(user.id, {
				display_name:
					edit.display_name !== (user.display_name ?? '') ? edit.display_name : undefined,
				email:
					edit.email.trim() && edit.email.trim() !== user.email ? edit.email.trim() : undefined,
				password: edit.password || undefined
			});
			replace(updated);
			editingId = undefined;
		});
	}

	function signOut(id: Snowflake) {
		run(() => api.users.admin.signOut(id));
	}

	function remove(id: Snowflake) {
		run(async () => {
			await api.users.admin.delete(id);
			users = users.filter((u) => u.id !== id);
			confirmingDelete = undefined;
		});
	}

	function createInvite() {
		run(async () => {
			const invite = await api.users.invites.create({
				max_uses: newInvite.max_uses === '' ? undefined : newInvite.max_uses,
				expires_at:
					newInvite.days === ''
						? undefined
						: new Date(Date.now() + newInvite.days * 86_400_000).toISOString(),
				role_id: newInvite.role_id || undefined
			});
			const { code, ...rest } = invite;
			invites = [rest, ...invites];
			createdCode = code;
		});
	}

	function revokeInvite(id: Snowflake) {
		run(async () => {
			await api.users.invites.delete(id);
			invites = invites.filter((i) => i.id !== id);
		});
	}

	function copyCode() {
		navigator.clipboard.writeText(createdCode);
		showToast({
			title: t.settings.accounts.copied,
			message: t.settings.accounts.copiedMessage,
			variant: 'info',
			durationMs: 3500
		});
	}

	function inviteStatus(invite: Invite): 'active' | 'expired' | 'usedUp' {
		if (invite.expires_at && new Date(invite.expires_at) < new Date()) return 'expired';
		if (invite.max_uses !== null && invite.uses >= invite.max_uses) return 'usedUp';
		return 'active';
	}

	function usesLabel(invite: Invite) {
		return invite.max_uses === null
			? t.settings.accounts.used({ uses: invite.uses })
			: t.settings.accounts.usesOf({ uses: invite.uses, max: invite.max_uses });
	}

	function expiryLabel(invite: Invite) {
		if (!invite.expires_at) return t.settings.accounts.noExpiry;
		const days = Math.ceil((new Date(invite.expires_at).getTime() - Date.now()) / 86_400_000);
		if (days <= 0) return date(invite.expires_at);
		return t.settings.accounts.daysLeft({ days });
	}

	onMount(() => {
		if (!canManage) {
			loaded = true;
			return;
		}
		run(async () => {
			[users, invites, roles] = await Promise.all([
				api.users.admin.list(),
				api.users.invites.list(),
				api.roles.list()
			]);
			loaded = true;
		});
	});
</script>

<div class="page">
	<div class="topbar">
		<h2>{t.settings.accounts.title}</h2>
		{#if canManage}
			<button type="button" class="primary" onclick={() => (creating = !creating)} disabled={busy}>
				<Plus size={16} />
				{t.settings.accounts.newUser}
			</button>
		{/if}
	</div>

	{#if !canManage}
		<div class="status">{t.settings.accounts.noPermission}</div>
	{:else if !loaded}
		<div class="status">{error || t.settings.accounts.loading}</div>
	{:else}
		<div class="content">
			{#if error}<p class="error">{error}</p>{/if}

			{#if creating}
				<form
					class="card"
					onsubmit={(e) => {
						e.preventDefault();
						createUser();
					}}
				>
					<div class="row">
						<label class="field grow">
							<span class="label">{t.settings.accounts.username}</span>
							<input type="text" bind:value={newUser.username} />
						</label>
						<label class="field grow">
							<span class="label">{t.settings.accounts.emailOptional}</span>
							<input type="email" bind:value={newUser.email} />
						</label>
						<label class="field grow">
							<span class="label">{t.settings.accounts.password}</span>
							<input type="password" autocomplete="new-password" bind:value={newUser.password} />
						</label>
					</div>
					<div class="actions">
						<div class="spacer"></div>
						<button type="button" class="ghost" onclick={() => (creating = false)}
							>{t.common.cancel}</button
						>
						<button
							type="submit"
							class="primary"
							disabled={busy || !newUser.username.trim() || !newUser.password}
						>
							{t.settings.accounts.create}
						</button>
					</div>
				</form>
			{/if}

			<h3>{t.settings.accounts.users}</h3>
			<ul class="list">
				{#each users as user (user.id)}
					<li class="card">
						<div class="line">
							<span class="name">{user.display_name ?? user.username}</span>
							<span class="hint">@{user.username}</span>
							{#if user.email}<span class="hint">{user.email}</span>{/if}
							<span class="hint">{t.settings.accounts.joined({ date: date(user.created_at) })}</span
							>
							<div class="spacer"></div>
							<button
								type="button"
								class="icon"
								aria-label={t.settings.accounts.editNamed({ name: user.username })}
								{@attach tooltip(t.common.edit)}
								onclick={() => (editingId === user.id ? (editingId = undefined) : startEdit(user))}
							>
								<Pencil size={14} />
							</button>
							<button
								type="button"
								class="icon"
								aria-label={t.settings.accounts.signOutNamed({ name: user.username })}
								{@attach tooltip(t.settings.accounts.signOut)}
								onclick={() => signOut(user.id)}
								disabled={busy}
							>
								<LogOut size={14} />
							</button>
							{#if confirmingDelete === user.id}
								<button
									type="button"
									class="danger"
									onclick={() => remove(user.id)}
									disabled={busy}
								>
									{t.settings.accounts.confirmDelete({ name: user.username })}
								</button>
								<button type="button" class="ghost" onclick={() => (confirmingDelete = undefined)}>
									{t.settings.accounts.keep}
								</button>
							{:else}
								<button
									type="button"
									class="icon danger-icon"
									aria-label={t.settings.accounts.deleteNamed({ name: user.username })}
									{@attach tooltip(t.settings.accounts.delete)}
									onclick={() => (confirmingDelete = user.id)}
								>
									<Trash2 size={14} />
								</button>
							{/if}
						</div>

						{#if editingId === user.id}
							<form
								class="row"
								onsubmit={(e) => {
									e.preventDefault();
									saveEdit(user);
								}}
							>
								<label class="field grow">
									<span class="label">{t.settings.accounts.displayName}</span>
									<input type="text" bind:value={edit.display_name} />
								</label>
								<label class="field grow">
									<span class="label">{t.settings.accounts.email}</span>
									<input type="email" bind:value={edit.email} />
								</label>
								<label class="field grow">
									<span class="label">{t.settings.accounts.newPassword}</span>
									<input
										type="password"
										autocomplete="new-password"
										placeholder={t.settings.accounts.unchanged}
										bind:value={edit.password}
									/>
								</label>
								<button type="submit" class="primary" disabled={busy}>{t.common.save}</button>
							</form>
							<p class="hint">{t.settings.accounts.passwordNote}</p>
						{/if}
					</li>
				{/each}
			</ul>

			<h3>{t.settings.accounts.invites}</h3>
			<p class="hint">
				{t.settings.accounts.invitesNote}
			</p>
			<form
				class="row"
				onsubmit={(e) => {
					e.preventDefault();
					createInvite();
				}}
			>
				<div class="field">
					<span class="label">{t.settings.accounts.uses}</span>
					<NumberField
						value={newInvite.max_uses}
						min={1}
						width="7rem"
						ariaLabel={t.settings.accounts.uses}
						placeholder={t.settings.accounts.unlimited}
						onchange={(v) => (newInvite.max_uses = v)}
					/>
				</div>
				<div class="field">
					<span class="label">{t.settings.accounts.validFor}</span>
					<NumberField
						value={newInvite.days}
						min={1}
						width="7rem"
						ariaLabel={t.settings.accounts.validFor}
						placeholder={t.settings.accounts.never}
						onchange={(v) => (newInvite.days = v)}
					/>
				</div>
				{#if assignableRoles.length > 0}
					<label class="field">
						<span class="label">{t.settings.accounts.role}</span>
						<select bind:value={newInvite.role_id}>
							<option value="">{t.settings.accounts.none}</option>
							{#each assignableRoles as role (role.id)}
								<option value={role.id}>{role.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				<button type="submit" class="ghost" disabled={busy}
					>{t.settings.accounts.createInvite}</button
				>
			</form>

			{#if createdCode}
				<div class="card code">
					<code>{createdCode}</code>
					<button
						type="button"
						class="icon"
						{@attach tooltip(t.settings.accounts.copyCode)}
						onclick={copyCode}
					>
						<Copy size={14} />
					</button>
					<span class="hint">{t.settings.accounts.copyNow}</span>
					<div class="spacer"></div>
					<button
						type="button"
						class="icon"
						{@attach tooltip(t.common.dismiss)}
						onclick={() => (createdCode = '')}
					>
						<X size={14} />
					</button>
				</div>
			{/if}

			<h3>{t.settings.accounts.activeInvites}</h3>
			{#if invites.length === 0}
				<p class="hint">{t.settings.accounts.noInvites}</p>
			{:else}
				<ul class="list">
					{#each invites as invite (invite.id)}
						{@const status = inviteStatus(invite)}
						<li class="card line invite" class:inactive={status !== 'active'}>
							<Badge tone={status === 'active' ? 'success' : 'neutral'}
								>{t.settings.accounts.status[status]}</Badge
							>
							<span class="fact" {@attach tooltip(t.settings.accounts.uses)}>
								<Users size={14} />
								{usesLabel(invite)}
							</span>
							<span class="fact" {@attach tooltip(t.settings.accounts.expiry)}>
								<Clock size={14} />
								{expiryLabel(invite)}
							</span>
							{#if invite.role_id}
								<span class="role-pill" {@attach tooltip(t.settings.accounts.roleOnSignup)}>
									<Shield />
									{roleName(invite.role_id)}
								</span>
							{/if}
							<div class="spacer"></div>
							<span class="hint"
								>{t.settings.accounts.created({ date: date(invite.created_at) })}</span
							>
							<ConfirmDelete
								prompt={t.settings.accounts.confirmDeleteInvite}
								onConfirm={() => revokeInvite(invite.id)}
							>
								{#snippet trigger(arm)}
									<button
										type="button"
										class="icon danger-icon"
										{@attach tooltip(t.settings.accounts.deleteInvite)}
										onclick={arm}
										disabled={busy}
									>
										<Trash2 size={14} />
									</button>
								{/snippet}
							</ConfirmDelete>
						</li>
					{/each}
				</ul>
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
	.content {
		flex: 1 1 auto;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
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
		background: none;
		color: var(--color-text-primary);
	}
	.danger-icon:hover {
		color: var(--color-error-text);
	}
	h3 {
		margin: var(--spacing-sm) 0 0;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
	}
	.line,
	.code {
		flex-direction: row;
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.invite {
		gap: var(--spacing-md);
		padding: var(--spacing-sm) var(--spacing-md);
	}
	.invite.inactive {
		opacity: 0.6;
	}
	.fact {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.role-pill {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-xs);
		padding: 2px var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--color-selected);
		color: var(--color-text-accent);
	}
	.role-pill :global(svg) {
		display: block;
		height: var(--font-size-sm);
		width: var(--font-size-sm);
	}
	.code code {
		user-select: all;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--spacing-md);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.grow {
		flex: 1 1 10rem;
	}
	.label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.spacer {
		flex: 1 1 auto;
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
</style>
