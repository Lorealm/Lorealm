import adapter from '@sveltejs/adapter-auto';
import { sveltekit } from '@sveltejs/kit/vite';
import { relative, sep } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) => {
					const relativePath = relative(import.meta.dirname, filename);
					const pathSegments = relativePath.toLowerCase().split(sep);
					const isExternalLibrary = pathSegments.includes('node_modules');

					return isExternalLibrary ? undefined : true;
				}
			},
			// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
			// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
			// See https://svelte.dev/docs/kit/adapters for more information about adapters.
			adapter: adapter(),
			files: {
				routes: 'dev/routes'
			}
		})
	]
});
