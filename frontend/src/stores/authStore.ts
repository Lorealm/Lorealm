import { defineStore } from 'pinia'
import api from '@/services/api'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    csrfToken: null,
  }),
  actions: {
    async fetchCsrfToken() {
      try {
        const response = await api.get('/user/csrf-token', {
          withCredentials: true,
          withXSRFToken: false,
        })
        this.csrfToken = response.data
      } catch (error) {
        console.error('Failed to fetch CSRF token:', error)
      }
    },
    clearCsrfToken() {
      this.csrfToken = null
    },
  },
})
