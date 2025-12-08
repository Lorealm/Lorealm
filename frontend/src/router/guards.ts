import type { Router } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

export function setupAuthGuards(router: Router): void {
  router.beforeEach(async (to, from, next) => {
    const authStore = useAuthStore()
    console.log('Guard: Navigating to:', to.name, 'from:', from.name, 'isAuthenticated:', authStore.isAuthenticated)
    const publicRoutes = ['login', 'register']
    if (publicRoutes.includes(to.name as string)) {
      console.log('Guard: Allowing public route:', to.name)
      return next()
    }
    if (!authStore.isAuthenticated) {
      console.log('Guard: Not authenticated, attempting hydrate...')
      await authStore.hydrate()
      console.log('Guard: After hydrate, isAuthenticated:', authStore.isAuthenticated)
    }
    if (!authStore.isAuthenticated) {
      console.log('Guard: Still not authenticated, redirecting to login')
      return next({
        name: 'login',
        query: { redirect: to.fullPath },
      })
    }
    console.log('Guard: Authenticated, proceeding to:', to.name)
    next()
  })
}