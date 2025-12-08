<script setup lang="ts">
import { ref, computed } from 'vue'
import EditTemplateView from './EditTemplateView.vue'
import MyTemplatesView from './MyTemplatesView.vue'
import '@assets/styles/views/context-view.css'

// ==================== Navigation State ====================

type ViewName = 'overview' | 'edit' | 'list' | 'lorebook-overview' | 'lorebook-edit' | 'lorebook-list' | 'macros' | 'functions' | 'debug'

const currentView = ref<ViewName>('list')
const editingTemplateId = ref<string | null>(null)

// ==================== Computed ====================

const isTemplateSection = computed(() => 
  ['overview', 'edit', 'list'].includes(currentView.value)
)

const isLorebookSection = computed(() => 
  ['lorebook-overview', 'lorebook-edit', 'lorebook-list'].includes(currentView.value)
)

const isAdvancedSection = computed(() => 
  ['macros', 'functions', 'debug'].includes(currentView.value)
)

// ==================== Navigation ====================

function navigateTo(view: ViewName): void {
  // If navigating away from edit, clear the template ID
  if (currentView.value === 'edit' && view !== 'edit') {
    editingTemplateId.value = null
  }
  currentView.value = view
}

function editTemplate(templateId: string): void {
  editingTemplateId.value = templateId
  currentView.value = 'edit'
}

function backToList(): void {
  editingTemplateId.value = null
  currentView.value = 'list'
}
</script>

<template>
  <div class="context-view">
    <!-- Sidebar Navigation -->
    <div class="context-sidebar">
      <div class="context-nav-section">
        <div class="context-nav-title">Templates</div>
        <button 
          class="context-nav-item" 
          :class="{ active: currentView === 'overview' }"
          @click="navigateTo('overview')"
        >
          <span>📊</span> Overview
        </button>
        <button 
          class="context-nav-item" 
          :class="{ active: currentView === 'edit' }"
          @click="navigateTo('edit')"
        >
          <span>✏️</span> Edit Template
        </button>
        <button 
          class="context-nav-item" 
          :class="{ active: currentView === 'list' }"
          @click="navigateTo('list')"
        >
          <span>📁</span> My Templates
        </button>
      </div>
      <div class="context-nav-section">
        <div class="context-nav-title">Lorebook</div>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'lorebook-overview' }"
          @click="navigateTo('lorebook-overview')"
        >
          <span>📚</span> Overview
        </button>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'lorebook-edit' }"
          @click="navigateTo('lorebook-edit')"
        >
          <span>📝</span> Edit Lorebook
        </button>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'lorebook-list' }"
          @click="navigateTo('lorebook-list')"
        >
          <span>📖</span> My Lorebooks
        </button>
      </div>
      <div class="context-nav-section">
        <div class="context-nav-title">Advanced</div>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'macros' }"
          @click="navigateTo('macros')"
        >
          <span>⚡</span> Macros
        </button>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'functions' }"
          @click="navigateTo('functions')"
        >
          <span>🔧</span> Functions
        </button>
        <button 
          class="context-nav-item"
          :class="{ active: currentView === 'debug' }"
          @click="navigateTo('debug')"
        >
          <span>🐛</span> Debug
        </button>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="context-content">
      <!-- Templates Section -->
      <template v-if="currentView === 'overview'">
        <div class="placeholder-view">
          <div class="placeholder-icon">📊</div>
          <h2>Templates Overview</h2>
          <p>Coming soon: Analytics and overview of your templates.</p>
        </div>
      </template>
      
      <template v-else-if="currentView === 'edit'">
        <EditTemplateView 
          :template-id="editingTemplateId ?? undefined"
          @back="backToList"
        />
      </template>
      
      <template v-else-if="currentView === 'list'">
        <MyTemplatesView @edit="editTemplate" />
      </template>
      
      <!-- Lorebook Section -->
      <template v-else-if="currentView === 'lorebook-overview'">
        <div class="placeholder-view">
          <div class="placeholder-icon">📚</div>
          <h2>Lorebook Overview</h2>
          <p>Coming soon: Analytics and overview of your lorebooks.</p>
        </div>
      </template>
      
      <template v-else-if="currentView === 'lorebook-edit'">
        <div class="placeholder-view">
          <div class="placeholder-icon">📝</div>
          <h2>Edit Lorebook</h2>
          <p>Coming soon: Lorebook editor.</p>
        </div>
      </template>
      
      <template v-else-if="currentView === 'lorebook-list'">
        <div class="placeholder-view">
          <div class="placeholder-icon">📖</div>
          <h2>My Lorebooks</h2>
          <p>Coming soon: Manage your lorebooks.</p>
        </div>
      </template>
      
      <!-- Advanced Section -->
      <template v-else-if="currentView === 'macros'">
        <div class="placeholder-view">
          <div class="placeholder-icon">⚡</div>
          <h2>Macros</h2>
          <p>Coming soon: Create and manage macros.</p>
        </div>
      </template>
      
      <template v-else-if="currentView === 'functions'">
        <div class="placeholder-view">
          <div class="placeholder-icon">🔧</div>
          <h2>Functions</h2>
          <p>Coming soon: Custom functions for templates.</p>
        </div>
      </template>
      
      <template v-else-if="currentView === 'debug'">
        <div class="placeholder-view">
          <div class="placeholder-icon">🐛</div>
          <h2>Debug</h2>
          <p>Coming soon: Debug and test your context templates.</p>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.context-view {
  display: flex;
  height: 100%;
  background: var(--color-bg-base);
}

.context-sidebar {
  width: 220px;
  min-width: 200px;
  background: var(--color-bg-elevated);
  border-right: 1px solid var(--color-border-subtle);
  padding: var(--spacing-lg) 0;
  overflow-y: auto;
}

.context-nav-section {
  margin-bottom: var(--spacing-xl);
}

.context-nav-title {
  padding: var(--spacing-sm) var(--spacing-lg);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--color-text-secondary);
}

.context-nav-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  width: 100%;
  padding: var(--spacing-sm) var(--spacing-lg);
  background: none;
  border: none;
  text-align: left;
  font-size: var(--font-size-sm);
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.context-nav-item:hover {
  background: var(--color-hover);
}

.context-nav-item.active {
  background: var(--color-active);
  color: var(--color-primary);
  font-weight: var(--font-weight-medium);
}

.context-nav-item span {
  font-size: var(--font-size-base);
  width: 20px;
  text-align: center;
}

.context-content {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* Placeholder for unimplemented views */
.placeholder-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: var(--spacing-2xl);
  color: var(--color-text-secondary);
}

.placeholder-icon {
  font-size: 64px;
  margin-bottom: var(--spacing-lg);
  opacity: 0.5;
}

.placeholder-view h2 {
  margin: 0 0 var(--spacing-sm) 0;
  font-size: var(--font-size-2xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.placeholder-view p {
  margin: 0;
  font-size: var(--font-size-md);
  max-width: 400px;
}
</style>