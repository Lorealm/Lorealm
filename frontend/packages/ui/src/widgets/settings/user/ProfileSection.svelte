<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { daysSince, monthYear, relativeTime } from '#lib/assets/scripts/datetime.js';
	import { blobUrl } from '#lib/api/blobs.js';
	import { currentUser } from '#lib/stores/currentUser.svelte.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
	import { realmStore } from '#lib/stores/realm.svelte.js';
	import { showToast } from '#lib/stores/errorToast.svelte.js';
	import { extractError } from '#lib/types/error.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { UserSession } from '#lib/types/user.js';
	import { getPlatformApi } from '#lib/core/context.js';

	const user = $derived(currentUser.user);
	const avatar = $derived(blobUrl(user?.avatar_uri));

	let nameDraft = $state('');
	let editingName = $state(false);
	let avatarInput = $state<HTMLInputElement | null>(null);
	let busy = $state(false);

	const memberSince = $derived(user ? monthYear(user.created_at) : null);

	const stats = $derived([
		{ label: t.settings.profile.stats.characters, value: characterStore.metadataList?.length ?? 0 },
		{ label: t.settings.profile.stats.lorebooks, value: lorebookStore.metadataList?.length ?? 0 },
		{ label: t.settings.profile.stats.realms, value: realmStore.summaries?.length ?? 0 },
		{
			label: t.settings.profile.stats.daysHere,
			value: user ? daysSince(user.created_at) : 0
		}
	]);

	$effect(() => {
		void currentUser.loadSessions().catch(report);
	});

	function report(e: unknown) {
		showToast({ message: extractError(e).message, variant: 'error' });
	}

	function startRename() {
		nameDraft = user?.display_name ?? '';
		editingName = true;
	}

	async function commitRename() {
		editingName = false;
		if (nameDraft === (user?.display_name ?? '')) return;
		try {
			await currentUser.setDisplayName(nameDraft);
		} catch (e) {
			report(e);
		}
	}

	async function pickAvatar(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		busy = true;
		try {
			await currentUser.setAvatar(file);
		} catch (e) {
			report(e);
		} finally {
			busy = false;
			if (avatarInput) avatarInput.value = '';
		}
	}

	const platform = getPlatformApi();

	async function signOut() {
		await platform.auth.logout();
		// Reload rather than reset auth state, so nothing from this account
		// stays in memory for whoever signs in next.
		location.reload();
	}

	let pwCurrent = $state('');
	let pwNew = $state('');
	let pwConfirm = $state('');
	let pwBusy = $state(false);
	let pwError = $state<string | null>(null);
	const pwMismatch = $derived(pwConfirm !== '' && pwNew !== pwConfirm);
	const pwReady = $derived(pwCurrent !== '' && pwNew !== '' && pwNew === pwConfirm);

	async function changePassword(event: SubmitEvent) {
		event.preventDefault();
		if (!pwReady || pwBusy) return;
		pwBusy = true;
		pwError = null;
		try {
			await currentUser.changePassword(pwCurrent, pwNew);
			pwCurrent = pwNew = pwConfirm = '';
			showToast({
				message: t.settings.profile.passwordChanged,
				variant: 'success'
			});
		} catch (e) {
			pwError = extractError(e).message;
		} finally {
			pwBusy = false;
		}
	}

	async function revoke(id: Snowflake) {
		try {
			await currentUser.revokeSession(id);
		} catch (e) {
			report(e);
		}
	}

	async function revokeOthers() {
		try {
			await currentUser.revokeOtherSessions();
		} catch (e) {
			report(e);
		}
	}

	function describe(session: UserSession): string {
		const ua = session.user_agent;
		if (!ua) return t.settings.profile.unknownDevice;

		const browser = /Edg\//.test(ua)
			? 'Edge'
			: /OPR\//.test(ua)
				? 'Opera'
				: /Firefox\//.test(ua)
					? 'Firefox'
					: /Chrome\//.test(ua)
						? 'Chrome'
						: /Safari\//.test(ua)
							? 'Safari'
							: null;

		// Non-browser clients lead with their own product token, e.g. "Lorealm-Desktop/0.3.0".
		const [, name, version] = ua.match(/^([\w.-]+)\/(\S+)/) ?? [];
		const product = name && name !== 'Mozilla' ? `${name.replace(/[-_]/g, ' ')} ${version}` : null;
		const client = browser ?? product;

		// Case-insensitive so Rust's std::env::consts::OS values ("macos", "windows", ...) match too.
		const os = /windows/i.test(ua)
			? 'Windows'
			: /android/i.test(ua)
				? 'Android'
				: /iphone|ipad/i.test(ua)
					? 'iOS'
					: /mac os x|macos/i.test(ua)
						? 'macOS'
						: /linux/i.test(ua)
							? 'Linux'
							: null;

		if (client && os) return t.settings.profile.clientOn({ client, os });
		return client ?? os ?? t.settings.profile.unknownDevice;
	}

	const otherSessions = $derived(currentUser.sessions.filter((s) => !s.current).length);
</script>

<section class="profile">
	{#if user}
		<header class="identity">
			<button
				class="avatar"
				onclick={() => avatarInput?.click()}
				disabled={busy}
				{@attach tooltip(t.settings.profile.changePicture)}
			>
				{#if avatar}
					<img src={avatar} alt="" />
				{:else}
					<span class="initial">{currentUser.name.charAt(0).toUpperCase()}</span>
				{/if}
				<span class="avatarHint">{t.settings.profile.change}</span>
			</button>
			<input
				bind:this={avatarInput}
				type="file"
				accept="image/png,image/jpeg,image/webp,image/gif"
				onchange={pickAvatar}
				hidden
				style="display:none"
			/>

			<div class="who">
				{#if editingName}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						class="nameInput"
						bind:value={nameDraft}
						placeholder={user.username}
						maxlength="64"
						autofocus
						onblur={commitRename}
						onkeydown={(e) => {
							if (e.key === 'Enter') e.currentTarget.blur();
							if (e.key === 'Escape') editingName = false;
						}}
					/>
				{:else}
					<button
						class="name"
						onclick={startRename}
						{@attach tooltip(t.settings.profile.changeName)}
					>
						{currentUser.name}
					</button>
				{/if}
				<span class="handle">@{user.username}</span>
				{#if memberSince}
					<span class="since">{t.settings.profile.joined({ when: memberSince })}</span>
				{/if}
			</div>
		</header>

		<div class="stats">
			{#each stats as stat (stat.label)}
				<div class="stat">
					<span class="value">{stat.value}</span>
					<span class="label">{stat.label}</span>
				</div>
			{/each}
		</div>

		<form class="password" onsubmit={changePassword}>
			<h3>{t.settings.profile.password}</h3>
			<label for="pw-current">{t.settings.profile.currentPassword}</label>
			<input
				id="pw-current"
				type="password"
				autocomplete="current-password"
				bind:value={pwCurrent}
			/>
			<label for="pw-new">{t.settings.profile.newPassword}</label>
			<input id="pw-new" type="password" autocomplete="new-password" bind:value={pwNew} />
			<label for="pw-confirm">{t.settings.profile.confirmPassword}</label>
			<input id="pw-confirm" type="password" autocomplete="new-password" bind:value={pwConfirm} />
			{#if pwMismatch}
				<p class="pwMessage">{t.settings.profile.mismatch}</p>
			{:else if pwError}
				<p class="pwMessage">{pwError}</p>
			{/if}
			<button type="submit" disabled={!pwReady || pwBusy}>
				{pwBusy ? t.settings.profile.changing : t.settings.profile.changePassword}
			</button>
			<p class="pwNote">{t.settings.profile.passwordNote}</p>
		</form>

		<div class="sessions">
			<div class="sessionsHead">
				<h3>{t.settings.profile.sessions}</h3>
				{#if otherSessions > 0}
					<button class="linkish" onclick={revokeOthers}>
						{t.settings.profile.signOutOthers({ count: otherSessions })}
					</button>
				{/if}
			</div>

			<ul>
				{#each currentUser.sessions as session (session.id)}
					<li>
						<div class="device">
							<span class="deviceName">
								{describe(session)}
								{#if session.current}<span class="badge">{t.settings.profile.thisDevice}</span>{/if}
							</span>
							<span class="meta">
								{session.ip_address ?? t.settings.profile.unknownAddress} ·
								{relativeTime(session.last_rotated_at) ?? t.settings.profile.unknown}
							</span>
						</div>
						{#if session.current}
							<button class="linkish" onclick={signOut}>{t.settings.profile.signOut}</button>
						{:else}
							<button class="linkish" onclick={() => revoke(session.id)}
								>{t.settings.profile.signOut}</button
							>
						{/if}
					</li>
				{:else}
					<li class="empty">{t.settings.profile.noSessions}</li>
				{/each}
			</ul>
		</div>
	{/if}
</section>

<style>
	.profile {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-3xl);
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-xl) var(--spacing-lg);
	}
	.identity {
		display: flex;
		align-items: center;
		gap: var(--spacing-lg);
	}
	.avatar {
		position: relative;
		width: 72px;
		height: 72px;
		flex-shrink: 0;
		padding: 0;
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-full);
		background: var(--gradient-portrait);
		overflow: hidden;
		cursor: pointer;
	}
	.avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.initial {
		font-size: var(--font-size-h3);
		color: var(--color-text-white);
	}
	.avatarHint {
		position: absolute;
		inset: auto 0 0 0;
		padding: 2px 0;
		font-size: var(--font-size-xs);
		color: var(--color-text-white);
		background: var(--color-bg-overlay);
		opacity: 0;
		transition: opacity 120ms ease;
	}
	.avatar:hover .avatarHint,
	.avatar:focus-visible .avatarHint {
		opacity: 1;
	}
	.who {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.name,
	.nameInput {
		font-size: var(--font-size-h4);
		font-family: var(--font-family-heading);
		color: var(--color-text-primary);
		text-align: left;
		padding: 0;
		background: none;
		border: none;
	}
	.name {
		cursor: text;
	}
	.name:hover {
		color: var(--color-text-accent);
	}
	.nameInput {
		border-bottom: 1px solid var(--color-border-active);
	}
	.handle,
	.since {
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
		gap: var(--spacing-lg) var(--spacing-sm);
		padding: var(--spacing-lg) 0;
		border-block: 1px solid var(--color-border-subtle);
	}
	.stat {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.value {
		font-size: var(--font-size-h3);
		font-family: var(--font-family-heading);
		color: var(--color-text-primary);
		line-height: 1.1;
	}
	.label {
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.password {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--spacing-xs);
		max-width: 22rem;
	}
	.password input {
		width: 100%;
		margin-bottom: var(--spacing-xs);
	}
	.password label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.password button {
		margin-top: var(--spacing-xs);
	}
	.pwMessage {
		margin: 0;
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
	.pwNote {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.sessionsHead {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		margin-bottom: var(--spacing-sm);
	}
	h3 {
		font-size: var(--font-size-sm);
		font-weight: normal;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}
	.sessions ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.sessions li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
		padding: var(--spacing-sm) 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.sessions li:last-child {
		border-bottom: none;
	}
	.device {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.deviceName {
		color: var(--color-text-primary);
	}
	.badge {
		margin-left: var(--spacing-xs);
		padding: 1px 6px;
		border-radius: var(--radius-full);
		font-size: var(--font-size-xs);
		color: var(--color-text-accent);
		background: var(--color-selected);
	}
	.meta {
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.linkish {
		padding: 0;
		border: none;
		background: none;
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
		cursor: pointer;
	}
	.linkish:hover {
		color: var(--color-error-text);
	}
</style>
