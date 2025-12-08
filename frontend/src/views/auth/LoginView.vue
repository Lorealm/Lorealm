<template>
  <div class="flex-column items-center justify-center h-full w-full p-xl" style="background: var(--gradient-bg);">
    <div class="flex-column gap-xl w-90" style="max-width: 400px; background: var(--color-bg-panel); border: 1px solid var(--color-border-subtle); border-radius: var(--radius-xl); padding: var(--spacing-xl); box-shadow: var(--shadow-md);">
      <h1 class="text-center m-0" style="color: var(--color-text-white);">Login</h1>
      
      <form @submit.prevent="handleLogin" class="flex-column gap-lg">
        <div class="form-group">
          <label class="form-label">Username or Email</label>
          <input v-model="identifier" type="text" required class="form-input" placeholder="Enter your username or email" />
        </div>
        
        <div class="form-group">
          <label class="form-label">Password</label>
          <input v-model="password" type="password" required class="form-input" placeholder="Enter your password" />
        </div>
        
        <button type="submit" class="btn btn-primary" :disabled="isLoading">
          {{ isLoading ? 'Logging in...' : 'Login' }}
        </button>
        
        <div v-if="displayError" class="text-center mt-sm" :style="{ color: errorColor, fontSize: 'var(--font-size-sm)' }">
          {{ displayError }}
        </div>
      </form>
      
      <p class="text-center mt-md m-0" style="color: var(--color-text-secondary); font-size: var(--font-size-sm);">
        Don't have an account? <router-link to="/register" style="color: var(--color-primary);">Register</router-link>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuth } from '@/composables/useAuth'

const router = useRouter()
const route = useRoute()
const { login, isLoading, error, isAuthenticated, user, isCredentialError } = useAuth()

const identifier = ref('')
const password = ref('')

// Map error types to user-friendly messages
const errorMessageMap: Record<string, string> = {
  token_expired: 'Your session has expired. Please log in again.',
  session_expired: 'Your session has expired. Please log in again.',
  session_revoked: 'Your session has been revoked. Please log in again.',
  invalid_token: 'Your session is invalid. Please log in again.',
  missing_token: 'Authentication required. Please log in.',
  invalid_csrf_token: 'Security validation failed. Please refresh and try again.',
  missing_csrf_token: 'Security validation failed. Please refresh and try again.',
  device_mismatch: 'This device is not recognized. Please log in again.',
  invalid_user: 'Invalid email or password.',
  incorrect_password: 'Invalid email or password.',
  registration_error: 'Registration failed. Please try again.',
  permission_denied: 'You do not have permission to access this resource.',
  encryption_error: 'An internal error occurred. Please try again later.',
  cryptography_error: 'An internal error occurred. Please try again later.',
  persistence_error: 'An internal error occurred. Please try again later.',
  token_generation_failed: 'An internal error occurred. Please try again later.',
}

// Show reason why user was logged out (from query params)
const logoutReason = computed(() => {
  const reason = route.query.reason as string | undefined
  return reason ? errorMessageMap[reason] || 'Please log in.' : ''
})

// Determine which error message to show
const displayError = computed(() => {
  if (logoutReason.value) return logoutReason.value
  if (error.value?.errorType) return errorMessageMap[error.value.errorType] || error.value.message
  return ''
})

// Determine error severity for styling
const errorSeverity = computed(() => {
  if (!error.value?.errorType) return 'info'
  if (isCredentialError()) return 'warning'
  if (['permission_denied'].includes(error.value.errorType)) return 'error'
  if (['encryption_error', 'cryptography_error', 'persistence_error', 'token_generation_failed'].includes(error.value.errorType)) return 'error'
  return 'error'
})

// Compute error color based on severity
const errorColor = computed(() => {
  switch (errorSeverity.value) {
    case 'info':
      return 'var(--color-info-text, var(--color-text-secondary))'
    case 'warning':
      return 'var(--color-warning-text, var(--color-error-text))'
    default:
      return 'var(--color-error-text)'
  }
})

const handleLogin = async () => {
  console.log('Attempting login with:', { identifier: identifier.value, password: password.value })
  const success = await login(identifier.value, password.value)
  console.log('Login result:', success, 'isAuthenticated:', isAuthenticated, 'user:', user.value)
  if (success) {
    console.log('Login successful, redirecting to write...')
    router.push({ name: 'write' })
  } else {
    console.log('Login failed, error:', error.value)
  }
}
</script>