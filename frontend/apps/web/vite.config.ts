import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Proxy /api to the backend during dev so the browser sees same-origin
// requests. This keeps cookies simple: no CORS, no SameSite=None;Secure.
const BACKEND = "http://127.0.0.1:1551";

export default defineConfig({
	plugins: [svelte()],
	server: {
		proxy: {
			"/api": {
				target: BACKEND,
				changeOrigin: true,
				secure: false,
				// Strip the cookie Domain so it binds to the dev origin
				// (localhost) instead of 127.0.0.1, and the browser keeps it.
				cookieDomainRewrite: "",
			},
		},
	},
});
