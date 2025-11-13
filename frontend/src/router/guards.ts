import type { Router } from 'vue-router'
import { useAuth } from '@/composables/useAuth'

export function setupAuthGuards(router: Router): void {
  router.beforeEach(async (to, from, next) => {
    const { isAuthenticated, hydrate } = useAuth()

    // Public routes that don't require authentication
    const publicRoutes = ['login']

    // If going to a public route, allow it
    if (publicRoutes.includes(to.name as string)) {
      return next()
    }

    // Try to hydrate if we have a token but aren't authenticated yet
    if (!isAuthenticated.value && localStorage.getItem('access_token')) {
      await hydrate()
    }

    // If still not authenticated, redirect to login
    if (!isAuthenticated.value) {
      return next({
        name: 'login',
        query: { redirect: to.fullPath },
      })
    }

    next()
  })
}
