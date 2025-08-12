<template>
  <div class="status-container">
    <SmartTooltip :disabled="status !== 'online' || !statusDetails" :verticalOffset="10">
      <template #trigger>
        <div
          class="status-circle"
          :class="statusClass"
          :title="status !== 'online' ? statusText : ''"
        ></div>
      </template>

      <template #content>
        <div class="status-details">
          <div class="status-header">
            <span class="status-name">{{ statusDetails.status }}</span>
            <span class="status-version">v{{ statusDetails.version }}</span>
          </div>
          <div class="status-info">
            <div class="info-row">
              <span class="info-label">Plugins:</span>
              <span class="info-value">{{ statusDetails.plugins?.count || 0 }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">Uptime:</span>
              <span class="info-value">{{ formatUptime(statusDetails.uptime) }}</span>
            </div>
          </div>
        </div>
      </template>
    </SmartTooltip>

    <span v-if="showText" class="status-text">{{ statusText }}</span>
  </div>
</template>

<script>
import SmartTooltip from './SmartTooltip.vue'
import api from '@/services/api'

export default {
  name: 'BackendStatusIcon',
  components: {
    SmartTooltip,
  },
  props: {
    endpoint: {
      type: String,
      default: '/status',
    },
    checkInterval: {
      type: Number,
      default: 5000, // 5 seconds
    },
    showText: {
      type: Boolean,
      default: false,
    },
    timeout: {
      type: Number,
      default: 5000, // 5 seconds
    },
  },
  data() {
    return {
      status: 'checking', // 'online', 'offline', 'checking'
      statusDetails: null,
      intervalId: null,
    }
  },
  computed: {
    statusClass() {
      return {
        'status-online': this.status === 'online',
        'status-offline': this.status === 'offline',
        'status-checking': this.status === 'checking',
      }
    },
    statusText() {
      const statusMap = {
        online: 'Backend Online',
        offline: 'Backend Offline',
        checking: 'Checking...',
      }
      return statusMap[this.status]
    },
  },
  mounted() {
    this.checkStatus()
    this.startPeriodicCheck()
  },
  beforeUnmount() {
    if (this.intervalId) {
      clearInterval(this.intervalId)
    }
  },
  methods: {
    async checkStatus() {
      this.status = 'checking'

      try {
        const controller = new AbortController()
        const timeoutId = setTimeout(() => controller.abort(), this.timeout)

        const response = await api.get(this.endpoint)

        clearTimeout(timeoutId)

        if (response.data.status) {
          this.status = 'online'
          this.statusDetails = response.data
          this.$emit('status-change', 'online')
        } else {
          this.status = 'offline'
          this.statusDetails = null
          this.$emit('status-change', 'offline')
        }
      } catch (error) {
        this.status = 'offline'
        this.statusDetails = null
        this.$emit('status-change', 'offline')
        console.warn('Backend status check failed:', error.message)
      }
    },

    startPeriodicCheck() {
      this.intervalId = setInterval(() => {
        this.checkStatus()
      }, this.checkInterval)
    },

    // Manual refresh method
    refresh() {
      this.checkStatus()
    },

    formatUptime(seconds) {
      if (seconds < 60) return `${seconds}s`

      const minutes = Math.floor(seconds / 60)
      if (minutes < 60) return `${minutes}m`

      const hours = Math.floor(minutes / 60)
      if (hours < 24) return `${hours}h ${minutes % 60}m`

      const days = Math.floor(hours / 24)
      return `${days}d ${hours % 24}h`
    },
  },
}
</script>

<style scoped>
.status-container {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.status-circle {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  transition: all 0.3s ease;
  cursor: pointer;
}

.status-online {
  background-color: #22c55e;
  box-shadow: 0 0 0 2px rgba(34, 197, 94, 0.2);
}

.status-offline {
  background-color: #ef4444;
  box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.2);
}

.status-checking {
  background-color: #f59e0b;
  box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.2);
  animation: pulse 2s infinite;
}

.status-text {
  font-size: 14px;
  font-weight: 500;
  color: #374151;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

/* Hover effects */
.status-circle:hover {
  transform: scale(1.1);
}

/* Status Details Content */
.status-details {
  min-width: 200px;
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  padding-bottom: 8px;
  border-bottom: 1px solid #374151;
}

.status-name {
  font-weight: 600;
  color: #22c55e;
  text-transform: capitalize;
}

.status-version {
  font-size: 11px;
  color: #9ca3af;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.info-label {
  color: #d1d5db;
  font-weight: 500;
}

.info-value {
  color: #f3f4f6;
  font-weight: 600;
}
</style>
