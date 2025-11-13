<script setup lang="ts">
import EditTemplatePage from '@/pages/context/EditTemplatePage.vue';
</script>

<template>
  <div class="context-view">
    <!-- Sidebar Navigation -->
    <div class="context-sidebar">
      <div class="context-nav-section">
        <div class="context-nav-title">Templates</div>
        <button class="context-nav-item" @click="currentView = 'overview'">
          <span>📊</span> Overview
        </button>
        <button class="context-nav-item active" @click="currentView = 'edit'">
          <span>✏️</span> Edit Template
        </button>
        <button class="context-nav-item" @click="currentView = 'list'">
          <span>📁</span> My Templates
        </button>
      </div>
      <div class="context-nav-section">
        <div class="context-nav-title">Lorebook</div>
        <button class="context-nav-item"><span>📚</span> Overview</button>
        <button class="context-nav-item"><span>📝</span> Edit Lorebook</button>
        <button class="context-nav-item"><span>📖</span> My Lorebooks</button>
      </div>
      <div class="context-nav-section">
        <div class="context-nav-title">Advanced</div>
        <button class="context-nav-item"><span>⚡</span> Macros</button>
        <button class="context-nav-item"><span>🔧</span> Functions</button>
        <button class="context-nav-item"><span>🐛</span> Debug</button>
      </div>
    </div>

    <EditTemplatePage />
  </div>
</template>