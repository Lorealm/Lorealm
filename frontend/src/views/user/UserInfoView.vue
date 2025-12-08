<template>
  <div v-if="hasAccess">
    <h1>User Information</h1>
    <p>This is the user information page.</p>

    <p>Username: {{ username }}</p>
    <p>Role: {{ role }}</p>
    <p style="color: gray">ID: {{ id }}</p>
  </div>
  <div v-else>
    <h1>Access Denied</h1>
    <p>You do not have access to this page.</p>
    <p style="color: red">Error: {{ error }}</p>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

var error = ref('')
var hasAccess = ref(false)
var id = ref('')
var username = ref('')
var role = ref('')

// onMounted(async () => {
//   try {
//     // Get token from cookie
//     const res = await api.get('/user/me', { withCredentials: true })

//     hasAccess.value = true
//     console.log(res.data)
//     id.value = res.data.id
//     username.value = res.data.username
//     role.value = res.data.role
//   } catch (err: any) {
//     hasAccess.value = false
//     error.value = err.message || 'generic error'
//   }
// })
</script>
