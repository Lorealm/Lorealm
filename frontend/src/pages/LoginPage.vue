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
