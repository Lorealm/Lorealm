<script setup lang="ts">
import { ref, onMounted } from 'vue'
import HelloWorld from '@/components/HelloWorld.vue'
import { api } from '@/services/api'

const healthStatus = ref<string>('checking...')
const serverInfo = ref<any>(null)

onMounted(async () => {
  try {
    const response = await api.get('/health')
    healthStatus.value = response.data.status
    
    const statusResponse = await api.get('/status')
    serverInfo.value = statusResponse.data
  } catch (error) {
    healthStatus.value = 'error'
    console.error('Failed to fetch health status:', error)
  }
})
</script>

<template>
  <div class="home">
    <h1>Welcome to Eldoria Hub</h1>
    
    <HelloWorld msg="Your application is running!" />
    
    <div class="status-card">
      <h2>Server Status</h2>
      <p>Health: <span :class="['status', healthStatus]">{{ healthStatus }}</span></p>
      
      <div v-if="serverInfo" class="server-info">
        <p>Version: {{ serverInfo.version }}</p>
        <p>Uptime: {{ Math.floor(serverInfo.uptime / 60) }} minutes</p>
        <div class="features">
          <h3>Features:</h3>
          <ul>
            <li v-for="(enabled, feature) in serverInfo.features" :key="feature">
              {{ feature }}: <span :class="enabled ? 'enabled' : 'disabled'">{{ enabled ? 'Enabled' : 'Disabled' }}</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home {
  text-align: center;
}

h1 {
  font-size: 2.5rem;
  margin-bottom: 2rem;
  color: #42b883;
}

.status-card {
  background: #f9f9f9;
  border-radius: 8px;
  padding: 2rem;
  margin: 2rem auto;
  max-width: 600px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.status {
  font-weight: bold;
  text-transform: uppercase;
}

.status.healthy {
  color: #42b883;
}

.status.error {
  color: #e74c3c;
}

.status.checking {
  color: #f39c12;
}

.server-info {
  margin-top: 1rem;
  text-align: left;
}

.features {
  margin-top: 1rem;
}

.features ul {
  list-style: none;
  padding: 0;
}

.features li {
  padding: 0.25rem 0;
}

.enabled {
  color: #42b883;
}

.disabled {
  color: #95a5a6;
}
</style>