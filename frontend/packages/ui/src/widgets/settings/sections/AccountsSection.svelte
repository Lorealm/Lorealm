<script lang="ts">
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
		return roles.find((r) => r.id === id)?.name ?? 'Unknown role';
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
			title: 'Copied code',
			message: 'The invite code has been copied to your clipboard.',
			variant: 'info',
			durationMs: 3500
		});
	}

	function inviteStatus(invite: Invite): 'Active' | 'Expired' | 'Used up' {
		if (invite.expires_at && new Date(invite.expires_at) < new Date()) return 'Expired';
		if (invite.max_uses !== null && invite.uses >= invite.max_uses) return 'Used up';
		return 'Active';
	}

	function usesLabel(invite: Invite) {
		return invite.max_uses === null
			? `${invite.uses} used`
			: `${invite.uses} of ${invite.max_uses}`;
	}

	function expiryLabel(invite: Invite) {
		if (!invite.expires_at) return 'No expiry';
		const days = Math.ceil((new Date(invite.expires_at).getTime() - Date.now()) / 86_400_000);
		if (days <= 0) return date(invite.expires_at);
		return days === 1 ? '1 day left' : `${days} days left`;
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
		<h2>Accounts</h2>
		{#if canManage}
			<button type="button" class="primary" onclick={() => (creating = !creating)} disabled={busy}>
				<Plus size={16} />
				New user
			</button>
		{/if}
	</div>

	{#if !canManage}
		<div class="status">Managing accounts needs the Manage users permission.</div>
	{:else if !loaded}
		<div class="status">{error || 'Loading…'}</div>
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
							<span class="label">Username</span>
							<input type="text" bind:value={newUser.username} />
						</label>
						<label class="field grow">
							<span class="label">Email (optional)</span>
							<input type="email" bind:value={newUser.email} />
						</label>
						<label class="field grow">
							<span class="label">Password</span>
							<input type="password" autocomplete="new-password" bind:value={newUser.password} />
						</label>
					</div>
					<div class="actions">
						<div class="spacer"></div>
						<button type="button" class="ghost" onclick={() => (creating = false)}>Cancel</button>
						<button
							type="submit"
							class="primary"
							disabled={busy || !newUser.username.trim() || !newUser.password}
						>
							Create
						</button>
					</div>
				</form>
			{/if}

			<h3>Users</h3>
			<ul class="list">
				{#each users as user (user.id)}
					<li class="card">
						<div class="line">
							<span class="name">{user.display_name ?? user.username}</span>
							<span class="hint">@{user.username}</span>
							{#if user.email}<span class="hint">{user.email}</span>{/if}
							<span class="hint">Joined {date(user.created_at)}</span>
							<div class="spacer"></div>
							<button
								type="button"
								class="icon"
								aria-label="Edit {user.username}"
								{@attach tooltip('Edit')}
								onclick={() => (editingId === user.id ? (editingId = undefined) : startEdit(user))}
							>
								<Pencil size={14} />
							</button>
							<button
								type="button"
								class="icon"
								aria-label="Sign {user.username} out everywhere"
								{@attach tooltip('Sign out everywhere')}
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
									Delete {user.username} and everything they made?
								</button>
								<button type="button" class="ghost" onclick={() => (confirmingDelete = undefined)}>
									Keep
								</button>
							{:else}
								<button
									type="button"
									class="icon danger-icon"
									aria-label="Delete {user.username}"
									{@attach tooltip('Delete user')}
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
									<span class="label">Display name</span>
									<input type="text" bind:value={edit.display_name} />
								</label>
								<label class="field grow">
									<span class="label">Email</span>
									<input type="email" bind:value={edit.email} />
								</label>
								<label class="field grow">
									<span class="label">New password</span>
									<input
										type="password"
										autocomplete="new-password"
										placeholder="Unchanged"
										bind:value={edit.password}
									/>
								</label>
								<button type="submit" class="primary" disabled={busy}>Save</button>
							</form>
							<p class="hint">Setting a new password signs them out on every device.</p>
						{/if}
					</li>
				{/each}
			</ul>

			<h3>Invites</h3>
			<p class="hint">
				Anyone with a code can create an account, even while registration is turned off.
			</p>
			<form
				class="row"
				onsubmit={(e) => {
					e.preventDefault();
					createInvite();
				}}
			>
				<div class="field">
					<span class="label">Uses</span>
					<NumberField
						value={newInvite.max_uses}
						min={1}
						width="7rem"
						ariaLabel="Uses"
						placeholder="Unlimited"
						onchange={(v) => (newInvite.max_uses = v)}
					/>
				</div>
				<div class="field">
					<span class="label">Valid for (days)</span>
					<NumberField
						value={newInvite.days}
						min={1}
						width="7rem"
						ariaLabel="Valid for (days)"
						placeholder="Never"
						onchange={(v) => (newInvite.days = v)}
					/>
				</div>
				{#if assignableRoles.length > 0}
					<label class="field">
						<span class="label">Role</span>
						<select bind:value={newInvite.role_id}>
							<option value="">None</option>
							{#each assignableRoles as role (role.id)}
								<option value={role.id}>{role.name}</option>
							{/each}
						</select>
					</label>
				{/if}
				<button type="submit" class="ghost" disabled={busy}>Create invite</button>
			</form>

			{#if createdCode}
				<div class="card code">
					<code>{createdCode}</code>
					<button type="button" class="icon" {@attach tooltip('Copy code')} onclick={copyCode}>
						<Copy size={14} />
					</button>
					<span class="hint">Copy it now. It won't be shown again.</span>
					<div class="spacer"></div>
					<button
						type="button"
						class="icon"
						{@attach tooltip('Dismiss')}
						onclick={() => (createdCode = '')}
					>
						<X size={14} />
					</button>
				</div>
			{/if}

			<h3>Active Invites</h3>
			{#if invites.length === 0}
				<p class="hint">No invites yet.</p>
			{:else}
				<ul class="list">
					{#each invites as invite (invite.id)}
						{@const status = inviteStatus(invite)}
						<li class="card line invite" class:inactive={status !== 'Active'}>
							<Badge tone={status === 'Active' ? 'success' : 'neutral'}>{status}</Badge>
							<span class="fact" {@attach tooltip('Uses')}>
								<Users size={14} />
								{usesLabel(invite)}
							</span>
							<span class="fact" {@attach tooltip('Expiry')}>
								<Clock size={14} />
								{expiryLabel(invite)}
							</span>
							{#if invite.role_id}
								<span class="role-pill" {@attach tooltip('Role given on sign-up')}>
									<Shield />
									{roleName(invite.role_id)}
								</span>
							{/if}
							<div class="spacer"></div>
							<span class="hint">Created {date(invite.created_at)}</span>
							<button
								type="button"
								class="icon danger-icon"
								{@attach tooltip('Delete invite')}
								onclick={() => revokeInvite(invite.id)}
								disabled={busy}
							>
								<Trash2 size={14} />
							</button>
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
