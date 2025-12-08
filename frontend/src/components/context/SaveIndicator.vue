<script setup lang="ts">
import { computed } from 'vue'
import type { SaveState } from '@/types/context'

// ==================== Props ====================

const props = defineProps<{
  state: SaveState
  lastSavedAt: Date | null
  error?: string | null
}>()

// ==================== Computed ====================

const statusConfig = computed(() => {
  switch (props.state) {
    case 'saved':
      return {
        icon: '✓',
        text: 'Saved',
        class: 'status-saved',
      }
    case 'saving':
      return {
        icon: '↻',
        text: 'Saving...',
        class: 'status-saving',
      }
    case 'unsaved':
      return {
        icon: '●',
        text: 'Unsaved changes',
        class: 'status-unsaved',
      }
    case 'error':
      return {
        icon: '✕',
        text: props.error || 'Save failed',
        class: 'status-error',
      }
    default:
      return {
        icon: '?',
        text: 'Unknown',
        class: '',
      }
  }
})

const formattedTime = computed(() => {
  if (!props.lastSavedAt) return null
  
  const now = new Date()
  const diff = now.getTime() - props.lastSavedAt.getTime()
  const minutes = Math.floor(diff / 60000)
  
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes} min${minutes > 1 ? 's' : ''} ago`
  
  return props.lastSavedAt.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
  })
})
</script>

<template>
  <div class="save-indicator" :class="statusConfig.class">
    <span class="status-icon" :class="{ spinning: state === 'saving' }">
      {{ statusConfig.icon }}
    </span>
    <span class="status-text">{{ statusConfig.text }}</span>
    <span v-if="state === 'saved' && formattedTime" class="status-time">
      {{ formattedTime }}
    </span>
  </div>
</template>

<style scoped>
.save-indicator {
  display: inline-flex;
  align-items: center;
  gap: var(--spacing-xs);
  padding: var(--spacing-xs) var(--spacing-md);
  border-radius: var(--radius-full);
  font-size: var(--font-size-sm);
  transition: all var(--transition-fast);
}

.status-icon {
  font-size: var(--font-size-base);
  display: inline-block;
}

.status-icon.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.status-text {
  font-weight: var(--font-weight-medium);
}

.status-time {
  color: inherit;
  opacity: 0.7;
}

/* Status variants */
.status-saved {
  background: var(--color-success-bg);
  color: var(--color-success-text);
}

.status-saving {
  background: var(--color-info-bg);
  color: var(--color-info-text);
}

.status-unsaved {
  background: var(--color-warning-bg, rgba(255, 179, 0, 0.15));
  color: var(--color-warning, #ffb300);
}

.status-error {
  background: var(--color-error-bg);
  color: var(--color-error);
}
</style>