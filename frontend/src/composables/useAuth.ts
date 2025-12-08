import { computed } from 'vue'
import { useAuthStore } from '@/stores/auth'

export function useAuth() {
  const authStore = useAuthStore()

  return {
    user: computed(() => authStore.user),
    isAuthenticated: authStore.isAuthenticated,
    isLoading: authStore.isLoading,
    error: computed(() => authStore.error),
    login: authStore.login,
    register: authStore.register,
    logout: authStore.logout,
    hydrate: authStore.hydrate,
    clearError: authStore.clearError,
    isCredentialError: authStore.isCredentialError,
    isPermissionError: authStore.isPermissionError,
    isServerError: authStore.isServerError,
    isSessionTerminated: authStore.isSessionTerminated,
  }
}