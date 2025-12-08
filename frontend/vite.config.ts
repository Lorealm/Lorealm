import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@components': fileURLToPath(new URL('./src/components', import.meta.url)),
      '@views': fileURLToPath(new URL('./src/views', import.meta.url)),
      '@services': fileURLToPath(new URL('./src/services', import.meta.url)),
      '@stores': fileURLToPath(new URL('./src/stores', import.meta.url)),
      '@types': fileURLToPath(new URL('./src/types', import.meta.url)),
      '@utils': fileURLToPath(new URL('./src/utils', import.meta.url)),
      '@composables': fileURLToPath(new URL('./src/composables', import.meta.url)),
      '@assets': fileURLToPath(new URL('./src/assets', import.meta.url)),
    },
  },
  server: {
    host: true,
    port: 5173,
    proxy: {
      '/api': {
        target: process.env.VITE_API_URL || 'http://localhost:3000',
        changeOrigin: true,
        secure: false,
        rewrite: (path) => path.replace(/^\/api/, ''),
        configure: (proxy) => {
          proxy.on('proxyReq', (proxyReq, req) => {
            // Forward the client's IP
            let clientIp = req.headers['x-forwarded-for']
            if (Array.isArray(clientIp)) {
              clientIp = clientIp[0]
            } else if (clientIp) {
              clientIp = clientIp.split(',')[0].trim()
            } else {
              clientIp = req.socket.remoteAddress || ''
            }
            if (clientIp) {
              proxyReq.setHeader('X-Forwarded-For', clientIp)
            }

            // Ensure User-Agent is forwarded (should be default, but explicit)
            const userAgent = req.headers['user-agent']
            if (userAgent) {
              proxyReq.setHeader('User-Agent', userAgent)
            }
          })
        },
      },
      '/ws': {
        target: process.env.VITE_API_URL || 'http://localhost:3000',
        changeOrigin: true,
        secure: false,
        ws: true,
        rewrite: (path) => path.replace(/^\/ws/, ''),
        configure: (proxy) => {
          proxy.on('proxyReq', (proxyReq, req) => {
            // Same as above for WebSocket if needed
            let clientIp = req.headers['x-forwarded-for']
            if (Array.isArray(clientIp)) {
              clientIp = clientIp[0]
            } else if (clientIp) {
              clientIp = clientIp.split(',')[0].trim()
            } else {
              clientIp = req.socket.remoteAddress || ''
            }
            if (clientIp) {
              proxyReq.setHeader('X-Forwarded-For', clientIp)
            }

            const userAgent = req.headers['user-agent']
            if (userAgent) {
              proxyReq.setHeader('User-Agent', userAgent)
            }
          })
        },
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    sourcemap: true,
    rollupOptions: {
      output: {
        manualChunks: {
          vendor: ['vue', 'vue-router', 'pinia'],
          utils: ['axios'],
        },
      },
    },
  },
})
