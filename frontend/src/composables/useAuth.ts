import { ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { apiClient } from '@/api/client'
import type { User, LoginResponse, AuthError } from '@/types/auth'

export function useAuth() {
  const router = useRouter()
  const user = ref<User | null>(null)
  const isLoading = ref(false)
  const error = ref<AuthError | null>(null)

  const isAuthenticated = computed(() => !!user.value)

  // Error types that should trigger immediate logout and redirect to login
  const SESSION_TERMINATING_ERRORS = [
    'missing_token',
    'invalid_token',
    'session_expired',
    'session_revoked',
    'missing_csrf_token',
    'invalid_csrf_token',
    'device_mismatch',
  ]

  const CREDENTIAL_ERRORS = ['invalid_user', 'incorrect_password', 'registration_error']

  // Error types that indicate permission issues
  const PERMISSION_ERRORS = ['permission_denied']

  const SERVER_ERROR_TYPES = [
    'encryption_error',
    'cryptography_error',
    'persistence_error',
    'token_generation_failed',
  ]

  // Watch for session validation errors and redirect
  watch(error, (newError) => {
    if (!newError) return

    if (SESSION_TERMINATING_ERRORS.includes(newError.errorType ?? '')) {
      logout()
      router.push({
        name: 'login',
        query: { reason: newError.errorType },
      })
    }
  })

  const clearError = (): void => {
    error.value = null
  }

  const handleError = (err: unknown): void => {
    if (err instanceof Error) {
      const apiError = err as Error & { errorType?: string }
      error.value = {
        code: apiError.errorType ?? 'UNKNOWN_ERROR',
        message: err.message,
        errorType: apiError.errorType,
      }
    } else {
      error.value = {
        code: 'UNKNOWN_ERROR',
        message: 'An unexpected error occurred',
      }
    }
  }

  const login = async (email: string, password: string): Promise<boolean> => {
    isLoading.value = true
    clearError()

    try {
      const response = await apiClient.post<LoginResponse>('/auth/login', {
        email,
        password,
      })

      apiClient.setCsrfToken(response.csrf_token)
      const userData = await apiClient.get<{ user: User }>('/auth/me')
      user.value = userData.user

      return true
    } catch (err) {
      handleError(err)
      return false
    } finally {
      isLoading.value = false
    }
  }

  const register = async (email: string, password: string, name: string): Promise<boolean> => {
    isLoading.value = true
    clearError()

    try {
      const response = await apiClient.post<LoginResponse>('/auth/register', {
        email,
        password,
        name,
      })

      apiClient.setCsrfToken(response.csrf_token)
      const userData = await apiClient.get<{ user: User }>('/auth/me')
      user.value = userData.user

      return true
    } catch (err) {
      handleError(err)
      return false
    } finally {
      isLoading.value = false
    }
  }

  const logout = (): void => {
    user.value = null
    apiClient.setAccessToken('')
    apiClient.setCsrfToken('')
    localStorage.removeItem('access_token')
    localStorage.removeItem('csrf_token')
    clearError()
  }

  const hydrate = async (): Promise<void> => {
    try {
      const response = await apiClient.get<{
        user: User
        csrf_token?: string
      }>('/auth/me')

      user.value = response.user

      if (response.csrf_token) {
        apiClient.setCsrfToken(response.csrf_token)
      }
    } catch (err) {
      handleError(err)
      logout()
    }
  }

  // Helper to check error categories (useful for components)
  const isCredentialError = (): boolean => CREDENTIAL_ERRORS.includes(error.value?.errorType ?? '')

  const isPermissionError = (): boolean => PERMISSION_ERRORS.includes(error.value?.errorType ?? '')

  const isServerError = (): boolean => SERVER_ERROR_TYPES.includes(error.value?.errorType ?? '')

  const isSessionTerminated = (): boolean =>
    SESSION_TERMINATING_ERRORS.includes(error.value?.errorType ?? '')

  return {
    user: computed(() => user.value),
    isAuthenticated,
    isLoading,
    error: computed(() => error.value),
    login,
    register,
    logout,
    hydrate,
    clearError,
    isCredentialError,
    isPermissionError,
    isServerError,
    isSessionTerminated,
  }
}
