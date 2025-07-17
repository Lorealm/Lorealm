// plugins/HelloPlugin/vite.config.js
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import path from 'path'

export default defineConfig({
  plugins: [svelte()],
  build: {
    lib: {
      entry: path.resolve(__dirname, 'Component.svelte'),
      name: 'PluginComponent',
      fileName: () => 'component.js',
      formats: ['es'],
    },
    outDir: 'dist',
    rollupOptions: {
      external: ['svelte'],
    },
    emptyOutDir: true,
    minify: true,
  }
})
