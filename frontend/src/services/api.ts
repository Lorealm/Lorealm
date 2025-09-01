import axios, { type AxiosInstance } from 'axios'
import { useAuthStore } from '@/stores/authStore' // Adjust the import path as needed for your project structure

// Create axios instance with default config
export const api: AxiosInstance = axios.create({
  baseURL: '/api',
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
  withCredentials: true,
  withXSRFToken: true,
  xsrfHeaderName: 'X-CSRF-TOKEN',
})

// Request interceptor to add X-CSRF-TOKEN for mutating methods
api.interceptors.request.use(async (config) => {
  if (!config.withXSRFToken) {
    return config
  }
  const methodsNeedingCsrf = [
    'get' /* Temporary; Should probably be removed down the road */,
    'post',
    'put',
    'delete',
    'patch',
  ]
  if (methodsNeedingCsrf.includes(config.method?.toLowerCase() || '')) {
    const authStore = useAuthStore()
    if (!authStore.csrfToken) {
      await authStore.fetchCsrfToken()
    }
    if (authStore.csrfToken) {
      config.headers['X-CSRF-TOKEN'] = authStore.csrfToken
    }
  }
  return config
})

// Response interceptor
api.interceptors.response.use(
  (response) => {
    return response
  },
  (error) => {
    // Handle common errors
    if (error.response?.status === 401) {
      // Handle unauthorized access
      localStorage.removeItem('authToken')
      //window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)

// Export common API methods
export default {
  get: api.get,
  post: api.post,
  put: api.put,
  delete: api.delete,
  patch: api.patch,
}
