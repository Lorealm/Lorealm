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
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: linear-gradient(135deg, #1a1f2e 0%, #241b2f 100%);
  color: #e0def4;
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
  background: rgba(20, 22, 30, 0.8);
  padding: 14px 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(138, 127, 219, 0.1);
  backdrop-filter: blur(10px);
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo {
  width: 24px;
  height: 24px;
  background: linear-gradient(135deg, #8a7fdb 0%, #6366f1 100%);
  border-radius: 6px;
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
  color: #e0def4;
  font-size: 16px;
  font-weight: 500;
}

.tabs {
  display: flex;
  gap: 2px;
  background: rgba(0, 0, 0, 0.2);
  padding: 4px;
  border-radius: 8px;
}

.tab {
  color: #908caa;
  padding: 8px 16px;
  cursor: pointer;
  border-radius: 6px;
  font-size: 13px;
  transition: all 0.2s;
  background: transparent;
  border: none;
}

.tab:hover {
  background: rgba(255, 255, 255, 0.05);
}

.tab.active {
  background: rgba(138, 127, 219, 0.2);
  color: #e0def4;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

.status {
  color: #908caa;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  background: #10b981;
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
  background: rgba(138, 127, 219, 0.2);
  border: 1px solid rgba(138, 127, 219, 0.3);
  color: #e0def4;
  padding: 6px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.btn-settings:hover {
  background: rgba(138, 127, 219, 0.3);
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
  padding: 8px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-top: 1px solid rgba(138, 127, 219, 0.05);
  font-size: 12px;
  color: #908caa;
  z-index: 99;
}

.status-left,
.status-right {
  display: flex;
  gap: 20px;
}
</style>
