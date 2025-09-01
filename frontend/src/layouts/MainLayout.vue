<script lang="ts" setup>
import { ref, onMounted, provide } from 'vue'
import { Bell } from 'lucide-vue-next'
import { appStateKey } from '@/types/app'
import HealthIcon from '@/components/HealthIcon.vue'
import router from '@/router'

// Create reactive state properly
const messages = ref([
  { role: 'system', content: 'You are a helpful assistant.' },
  { role: 'assistant', content: 'Hello! What can I help you with today?' },
])
const userInput = ref('')

const app = {
  sidebarCollapsed: ref(false),
  modulesPanelCollapsed: ref(false),
  activeProfile: ref('Productivity'),
  activeModule: ref('Knowledge Base'),
  messages,
  userInput,
  profiles: [
    { id: 'Productivity', name: 'Productivity', icon: '💼' },
    { id: 'Creative', name: 'Creative Writing', icon: '✍️' },
    { id: 'RPG', name: 'Roleplay', icon: '🎭' },
    { id: 'Programming', name: 'Coding Assistant', icon: '💻' },
  ],
  modules: {
    Productivity: ['Knowledge Base', 'File Browser', 'Task Manager'],
    Creative: ['Character Profiles', 'Story Archives', 'Prompt Templates'],
    RPG: ['Character Sheets', 'World Settings', 'Scenario Builder'],
    Programming: ['Code Editor', 'Documentation', 'Snippets'],
  } as { [key: string]: string[] },
}

// Provide the app state - make sure this happens before ChatWindow is rendered
provide(appStateKey, app)

const dynamicComponents = ref<string[]>([])
const loadedComponents = new Map<string, any>()

function toggleSidebar() {
  app.sidebarCollapsed.value = !app.sidebarCollapsed.value
}

function toggleModulesPanel() {
  app.modulesPanelCollapsed.value = !app.modulesPanelCollapsed.value
}

function switchProfile(profileId: string) {
  app.activeProfile.value = profileId
  app.activeModule.value = app.modules[profileId][0]
}

function switchModule(moduleName: string) {
  app.activeModule.value = moduleName
}

onMounted(async () => {
  const components = new Array<string>()
  dynamicComponents.value = components

  for (const path of components) {
    try {
      const mod = await import(/* @vite-ignore */ path)
      loadedComponents.set(path, mod.default)
    } catch (err) {
      console.error(`Failed to load component ${path}`, err)
    }
  }
})
</script>

<template>
  <main class="app-container">
    <!-- Left Sidebar -->
    <aside class="sidebar" :class="{ collapsed: app.sidebarCollapsed.value }">
      <button class="toggle-btn" @click="toggleSidebar">
        {{ app.sidebarCollapsed.value ? '→' : '←' }}
      </button>

      <div v-if="!app.sidebarCollapsed.value">
        <div class="profiles-section">
          <h3>Profiles</h3>
          <ul class="profile-list">
            <li v-for="profile in app.profiles" :key="profile.id">
              <button
                :class="{ active: app.activeProfile.value === profile.id }"
                @click="switchProfile(profile.id)"
              >
                <span class="profile-icon">{{ profile.icon }}</span>
                <span class="profile-name">{{ profile.name }}</span>
              </button>
            </li>
          </ul>
        </div>

        <div class="modules-section">
          <h3>Active Modules</h3>
          <ul class="module-list">
            <li v-for="module in app.modules[app.activeProfile.value] || []" :key="module">
              <button
                :class="{ active: app.activeModule.value === module }"
                @click="switchModule(module)"
              >
                {{ module }}
              </button>
            </li>
          </ul>
        </div>

        <div class="settings-section">
          <RouterLink to="/settings">
            <button class="settings-btn">Settings</button>
          </RouterLink>
        </div>
      </div>
    </aside>

    <!-- Main Chat Area -->
    <RouterView />

    <!-- Right Modules Panel -->
    <aside class="modules-panel" :class="{ collapsed: app.modulesPanelCollapsed.value }">
      <button class="toggle-btn" @click="toggleModulesPanel">
        {{ app.modulesPanelCollapsed.value ? '←' : '→' }}
      </button>

      <div v-if="!app.modulesPanelCollapsed.value">
        <div class="module-header">
          <h2>{{ app.activeModule.value }}</h2>
        </div>

        <div class="module-content">
          <div v-if="app.activeModule.value === 'Knowledge Base'" class="knowledge-base-module">
            <input type="text" placeholder="Search knowledge base..." />
            <div class="kb-results">
              <p>Content would dynamically load here based on the active module.</p>
            </div>
          </div>

          <div
            v-else-if="app.activeModule.value === 'Character Profiles'"
            class="character-profiles-module"
          >
            <select>
              <option>Character 1</option>
              <option>Character 2</option>
            </select>
            <div class="character-details">
              <p>Character details would appear here.</p>
            </div>
          </div>

          <div v-else class="generic-module">
            <p>This is the {{ app.activeModule.value }} module interface.</p>
            <p>Module-specific controls would appear here.</p>
          </div>
        </div>

        <div id="extension-area" style="height: 500px; border-top: 1px solid #444">
          <div v-for="component in dynamicComponents" :key="component" class="extension-component">
            <component
              v-if="loadedComponents.get(component)"
              :is="loadedComponents.get(component)"
            />
            <div v-else class="extension-loading">Loading {{ component }}...</div>
          </div>
        </div>
      </div>
    </aside>

    <!-- Status Bar -->
    <footer class="status-bar">
      <div class="status-item model-info">Model: GPT-4 Turbo</div>
      <div class="status-item token-usage">Tokens: 145/4096</div>
      <div class="status-item settings-info">
        <button class="settings-btn">ℹ</button>
      </div>
      <div class="status-item connection-status">Connected ✓</div>
      <div class="status-item">
        <button class="round-button">
          <Bell />
        </button>
      </div>
      <div class="status-item">
        <HealthIcon />
      </div>
    </footer>
  </main>
</template>

<style scoped>
@import '@/assets/app.css';
</style>
