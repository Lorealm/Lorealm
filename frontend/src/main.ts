import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './assets/styles/index.css'
import { useAuthStore } from './stores/auth'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router) // Initialize router before hydration

// Hydrate auth state
const authStore = useAuthStore()
authStore.hydrate().then(() => {
  console.log('Auth state hydrated, mounting app')
  app.mount('#app')
}).catch((err) => {
  console.error('Hydration failed:', err)
  app.mount('#app')
})