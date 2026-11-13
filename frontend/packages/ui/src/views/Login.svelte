<script lang="ts">
	import ErrorToast from '#lib/components/ErrorToast.svelte';
	import { getPlatformApi } from '#lib/core/context.js';
	import { setAuthState } from '#lib/stores/auth.svelte.js';
	import type { LoginRequest } from '#lib/types/auth.js';
	import type { AppError, ServerError } from '#lib/types/error.js';
	import { memory } from '#lib/stores/memory.svelte.js';
	import { useApi } from '#lib/api/client.js';
	import { onMount } from 'svelte';

	const platform = getPlatformApi();
	/** Absent in the browser, which is bound to the origin it was served from. */
	const server = platform.server;

	let username = $state('');
	let password = $state('');
	let serverUrl = $state('');
	let error = $state('');
	let registering = $state(false);
	let email = $state('');
	let inviteCode = $state('');

	// This view mounts before the boot in App.svelte has read local config back.
	let prefilled = false;
	$effect(() => {
		if (!memory.loaded || prefilled) return;
		prefilled = true;
		username = memory.current.lastUsername;
	});

	onMount(() => {
		server?.url().then((url) => (serverUrl = url));
	});

	function showError(e: unknown) {
		if ((e as AppError).error_type == 'Server') {
			const data = (e as ServerError).data;
			error = `<b>${data.error_code}:</b> ${data.message}`;
			return;
		}
		error = JSON.stringify(e);
	}

	async function handleLogin() {
		try {
			if (server) await server.setUrl(serverUrl);
			const session = await platform.auth.login({ username, password } as LoginRequest);
			await memory.set('lastUsername', username);
			setAuthState({ status: 'authenticated', session });
		} catch (e: unknown) {
			showError(e);
		}
	}

	async function handleRegister() {
		try {
			if (server) await server.setUrl(serverUrl);
			await useApi().users.register({
				username,
				password,
				email: email.trim() || undefined,
				invite_code: inviteCode.trim() || undefined
			});
		} catch (e: unknown) {
			if (e instanceof Error) {
				error = e.message;
				return;
			}
			showError(e);
			return;
		}
		await handleLogin();
	}

	function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (registering) handleRegister();
		else handleLogin();
	}
</script>

<div class="container">
	<form class="login-form" onsubmit={handleSubmit}>
		<h1>Login Page</h1>
		{#if error}
			<div class="error-box">{@html error}</div>
		{/if}
		{#if server}
			<div class="input-row" style="margin-bottom: var(--spacing-xs);">
				<label for="ServerUrl">Server URL</label>
				<input id="ServerUrl" type="text" class="standard-input" bind:value={serverUrl} />
			</div>
		{/if}
		<div class="input-row" style="margin-bottom: var(--spacing-xs);">
			<label for="UsernameInput">Username</label>
			<input id="UsernameInput" type="text" class="standard-input" bind:value={username} />
		</div>
		<div class="input-row" style="margin-bottom: var(--spacing-md);">
			<label for="PasswordInput">Password</label>
			<input id="PasswordInput" type="password" class="standard-input" bind:value={password} />
		</div>
		{#if registering}
			<div class="input-row" style="margin-bottom: var(--spacing-xs);">
				<label for="EmailInput">Email (optional)</label>
				<input id="EmailInput" type="email" class="standard-input" bind:value={email} />
			</div>
			<div class="input-row" style="margin-bottom: var(--spacing-md);">
				<label for="InviteInput">Invite code</label>
				<input
					id="InviteInput"
					type="text"
					class="standard-input"
					placeholder="Needed if sign-ups are closed"
					bind:value={inviteCode}
				/>
			</div>
			<button type="submit" class="push-button">Create account</button>
			<button type="button" class="link-button" onclick={() => (registering = false)}>
				I already have an account
			</button>
		{:else}
			<button type="submit" class="push-button">Log in</button>
			<button type="button" class="link-button" onclick={() => (registering = true)}>
				Create an account
			</button>
		{/if}
	</form>
</div>

<ErrorToast />

<style>
	.container {
		display: flex;
		background-color: var(--color-bg-base);
		height: 100%;
		width: 100%;
		padding: 1em;
		overflow-y: auto;
	}
	.login-form {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: 28em;
		/* Auto margins center it but keep the top reachable when the window is short. */
		margin: auto;
	}
	.input-row {
		display: flex;
		flex-direction: row;
		justify-content: center;
		align-items: center;
	}
	.input-row label {
		min-width: 8em;
		flex-shrink: 0;
	}
	.link-button {
		align-self: center;
		margin-top: var(--spacing-sm);
		background: none;
		color: var(--color-text-secondary);
	}
	.error-box {
		padding: var(--spacing-md);
		margin-bottom: var(--spacing-lg);
		border-radius: var(--radius-md);
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
</style>
