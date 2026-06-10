import { mount } from 'svelte';
import { App } from "@lorealm/ui";
import { tauriPlatform } from './platform.js';

console.log("Setting up app...");

const app = mount(App, {
	target: document.getElementById('app')!,
	props: {
		api: tauriPlatform
	}
});

export default app;
