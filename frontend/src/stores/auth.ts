import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { apiClient } from '@/services/api/client'
import type { User, LoginResponse, AuthError } from '@/types/auth'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const isLoading = ref(false)
  const error = ref<AuthError | null>(null)
  const hasHydrated = ref(false) // Track if hydration has been attempted
  const router = useRouter()

  // Initialize CSRF token from localStorage
  const storedToken = localStorage.getItem('csrf_token')
  if (storedToken) {
    console.log('Restoring CSRF token from localStorage:', storedToken)
    apiClient.setCsrfToken(storedToken)
  }

  const isAuthenticated = computed(() => !!user.value)

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
  const PERMISSION_ERRORS = ['permission_denied']
  const SERVER_ERROR_TYPES = [
    'encryption_error',
    'cryptography_error',
    'persistence_error',
    'token_generation_failed',
  ]

  watch(error, (newError) => {
    if (!newError) return
    if (SESSION_TERMINATING_ERRORS.includes(newError.errorType ?? '')) {
      logout()
      // Ensure router is available before pushing
      if (router) {
        router.push({
          name: 'login',
          query: { reason: newError.errorType },
        })
      }
    }
  })

  const clearError = () => {
    error.value = null
  }

  const handleError = (err: unknown) => {
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

  const login = async (username: string, password: string): Promise<boolean> => {
    isLoading.value = true
    clearError()
    try {
      const response = await apiClient.post<LoginResponse>('/auth/login', {
        username,
        password,
      })
      console.log('Login API response:', response)
      if (!response.csrf_token) {
        throw new Error('No CSRF token received')
      }
      apiClient.setCsrfToken(response.csrf_token)
      const userData = await apiClient.get<User>('/auth/me')
      console.log('User data:', userData)
      if (!userData.id || !userData.username) {
        throw new Error('Invalid user data received from /auth/me')
      }
      user.value = userData
      console.log('Set user:', user.value)
      return true
    } catch (err) {
      console.error('Login error:', err)
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
      console.log('Register API response:', response)
      if (!response.csrf_token) {
        throw new Error('No CSRF token received')
      }
      apiClient.setCsrfToken(response.csrf_token)
      const userData = await apiClient.get<User>('/auth/me')
      console.log('User data:', userData)
      if (!userData.id || !userData.username) {
        throw new Error('Invalid user data received from /auth/me')
      }
      user.value = userData
      return true
    } catch (err) {
      console.error('Register error:', err)
      handleError(err)
      return false
    } finally {
      isLoading.value = false
    }
  }

  const logout = () => {
    user.value = null
    apiClient.setCsrfToken('')
    localStorage.removeItem('csrf_token')
    clearError()
    hasHydrated.value = false
  }

  const hydrate = async () => {
    if (hasHydrated.value) {
      console.log('Hydrate: Already attempted, skipping')
      return
    }
    isLoading.value = true
    try {
      const response = await apiClient.get<User & { csrf_token?: string }>('/auth/me')
      console.log('Hydrate API response:', response)
      if (!response.id || !response.username) {
        throw new Error('Invalid user data received from /auth/me')
      }
      user.value = { id: response.id, email: response.email, username: response.username }
      console.log('Hydrate updated user:', user.value)
      if (response.csrf_token) {
        apiClient.setCsrfToken(response.csrf_token)
        console.log('Set CSRF token:', response.csrf_token)
      }
      hasHydrated.value = true
    } catch (err) {
      console.error('Hydrate error:', err)
      handleError(err)
      logout()
    } finally {
      isLoading.value = false
    }
  }

  const isCredentialError = () => CREDENTIAL_ERRORS.includes(error.value?.errorType ?? '')
  const isPermissionError = () => PERMISSION_ERRORS.includes(error.value?.errorType ?? '')
  const isServerError = () => SERVER_ERROR_TYPES.includes(error.value?.errorType ?? '')
  const isSessionTerminated = () =>
    SESSION_TERMINATING_ERRORS.includes(error.value?.errorType ?? '')

  return {
    user,
    isAuthenticated,
    isLoading,
    error,
    hasHydrated,
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
})