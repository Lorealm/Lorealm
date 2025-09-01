<template>
  <div>
    <h1>Login</h1>
    <form @submit.prevent="login">
      <div>
        <label>Username:</label>
        <input v-model="username" type="text" required />
      </div>
      <div>
        <label>Password:</label>
        <input v-model="password" type="password" required />
      </div>
      <div>
        <button type="submit">Login</button>
      </div>
      <div v-if="error" style="color: red">
        {{ error }}
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import api from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

const username = ref('')
const password = ref('')
const error = ref('')

async function login() {
  error.value = ''
  try {
    const authStore = useAuthStore()
    const res = await api.post('/login', {
      username: username.value,
      password: password.value,
    })
    authStore.fetchCsrfToken()

    localStorage.setItem('user', res.data.user)
    window.location.href = '/' // redirect after login
  } catch (err: any) {
    error.value = err.response?.data?.message || 'Login failed'
  }
}
</script>
