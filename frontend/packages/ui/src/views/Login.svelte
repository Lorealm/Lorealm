<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { setAuthState } from '$lib/stores/auth.svelte';

	const api = useApi();
	let username = $state('');
	let password = $state('');
	let serverUrl = $state('');
	let error = $state('');

	async function handleLogin() {
		try {
			const session = await api.auth.login({ username, password });
			setAuthState({ status: 'authenticated', session });
		} catch (e) {
			error = 'Login failed: ' + JSON.stringify(e);
		}
	}
</script>

<div class="container">
	<h1>Login Page</h1>
	{#if error}
		<div class="error-box">{error}</div>
	{/if}
	<div class="input-row" style="margin-bottom: var(--spacing-xs);">
		<label for="ServerUrl">Server URL</label>
		<input id="ServerUrl" type="text" class="standard-input" bind:value={serverUrl} />
	</div>
	<div class="input-row" style="margin-bottom: var(--spacing-xs);">
		<label for="UsernameInput">Username</label>
		<input id="UsernameInput" type="text" class="standard-input" bind:value={username} />
	</div>
	<div class="input-row" style="margin-bottom: var(--spacing-md);">
		<label for="PasswordInput">Password</label>
		<input id="PasswordInput" type="password" class="standard-input" bind:value={password} />
	</div>
	<button class="push-button" onclick={handleLogin}>Log in</button>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		background-color: var(--color-bg-base);
		height: 100%;
		width: 100%;
		padding: 1em;
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
	.error-box {
		padding: var(--spacing-md);
		margin-bottom: var(--spacing-lg);
		border-radius: var(--radius-md);
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
</style>
