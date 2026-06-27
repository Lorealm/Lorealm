import { mount } from "svelte";
import { App } from "@lorealm/ui";
import { webPlatform } from "./platform.js";

console.log("Setting up app...");

const app = mount(App, {
	target: document.getElementById("app")!,
	props: {
		api: webPlatform,
	},
});

export default app;
