<script lang="ts">
import router from '@/router'

export default {
  methods: {
    switchTab(dest: string, btn: HTMLButtonElement) {
      const active_elements = document.getElementsByClassName('active')

      Array.from(active_elements).forEach((el) => {
        el.classList.remove('active')
      })

      btn.classList.add('active')
      router.push(dest)
    },
  },
}
</script>

<template>
  <div class="app-container">
    <div class="header">
      <div class="brand">
        <div class="logo"></div>
        <div class="title">Eldoria Hub</div>
      </div>
      <div class="tabs">
        <button class="tab active" @click="switchTab('write', $event.target as HTMLButtonElement)">
          Write
        </button>
        <button class="tab" @click="switchTab('context', $event.target as HTMLButtonElement)">
          Context
        </button>
        <button class="tab" @click="switchTab('characters', $event.target as HTMLButtonElement)">
          Characters
        </button>
        <button class="tab" @click="switchTab('worlds', $event.target as HTMLButtonElement)">
          World
        </button>
        <button class="tab" @click="switchTab('plugins', $event.target as HTMLButtonElement)">
          Plugins
        </button>
      </div>
      <div class="header-actions">
        <div class="status">
          <div class="status-dot"></div>
          Connected
        </div>
        <button class="btn-settings">Settings</button>
      </div>
    </div>
    <div class="main-content">
      <RouterView />
    </div>
    <div class="status-bar">
      <div class="status-left">
        <span>Words: 847</span>
        <span>Characters: 4,231</span>
        <span>Context: 71%</span>
        <span>Autosave: On</span>
      </div>
      <div class="status-right">
        <span>Model: GPT-4</span>
        <span>Temp: 0.8</span>
        <span>Max Tokens: 500</span>
      </div>
    </div>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-family-base);
  background: var(--gradient-bg);
  color: var(--color-text-primary);
  min-height: 100vh;
  overflow: hidden;
}

.app-container {
  max-width: 100vw;
  margin: 0 auto;
  height: 100vh;
  display: flex;
  flex-direction: column;
}

/* Header */
.header {
  background: var(--color-bg-panel);
  padding: 14px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--color-border-subtle);
  backdrop-filter: blur(10px);
}

.brand {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
}

.logo {
  width: var(--spacing-2xl);
  height: var(--spacing-2xl);
  background: var(--gradient-primary);
  border-radius: var(--radius-sm);
  animation: pulse 4s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.8;
  }
}

.title {
  color: var(--color-text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-medium);
}

.tabs {
  display: flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.2);
  padding: var(--spacing-xs);
  border-radius: var(--spacing-sm);
}

.tab {
  color: var(--color-text-secondary);
  padding: var(--spacing-sm) var(--spacing-lg);
  cursor: pointer;
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
  transition: all 0.2s;
  background: transparent;
  border: none;
}

.tab:hover {
  background: rgba(255, 255, 255, 0.05);
}

.tab.active {
  background: rgba(138, 127, 219, 0.2);
  color: var(--color-text-primary);
}

.header-actions {
  display: flex;
  gap: var(--spacing-md);
  align-items: center;
}

.status {
  color: var(--color-text-secondary);
  font-size: var(--spacing-md);
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  background: var(--color-success);
  border-radius: 50%;
  animation: statusPulse 2s ease-in-out infinite;
}

@keyframes statusPulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.btn-settings {
  background: var(--color-hover);
  border: 1px solid var(--color-active);
  color: var(--color-text-primary);
  padding: 6px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-size: var(--font-size-sm);
  transition: all 0.2s;
}

.btn-settings:hover {
  background: var(--color-active);
  transform: translateY(-1px);
}

/* Main Content Area */
.main-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Status Bar */
.status-bar {
  background: rgba(20, 22, 30, 0.8);
  padding: var(--spacing-sm) var(--spacing-xl);
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-top: 1px solid rgba(138, 127, 219, 0.05);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  z-index: var(--z-index-always-top);
}

.status-left,
.status-right {
  display: flex;
  gap: var(--spacing-xl);
}
</style>
