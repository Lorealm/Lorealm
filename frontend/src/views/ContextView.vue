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

<style>
/* All your existing styles remain the same, just adding some improvements */

.context-view {
  display: flex;
  flex-direction: row;
  height: 100vh;
  width: 100%;
  background: rgba(30, 33, 43, 0.95);
}

.context-sidebar {
  width: 250px;
  background: rgba(20, 22, 30, 0.6);
  border-right: 1px solid rgba(138, 127, 219, 0.1);
  padding: 20px;
  overflow-y: auto;
}

.context-nav-section {
  margin-bottom: 24px;
}

.context-nav-title {
  color: #8a7fdb;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 12px;
  opacity: 0.8;
}

.context-nav-item {
  color: #e0def4;
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 6px;
  font-size: 13px;
  margin: 4px 0;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 10px;
  background: transparent;
  border: none;
  width: 100%;
  text-align: left;
}

.context-nav-item:hover {
  background: rgba(138, 127, 219, 0.1);
}

.context-nav-item.active {
  background: rgba(138, 127, 219, 0.2);
}

.template-editor-container {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.template-hierarchy {
  width: 350px;
  background: rgba(20, 22, 30, 0.4);
  border-right: 1px solid rgba(138, 127, 219, 0.1);
  display: flex;
  flex-direction: column;
}

.hierarchy-header {
  padding: 20px;
  border-bottom: 1px solid rgba(138, 127, 219, 0.1);
}

.template-title {
  font-size: 18px;
  font-weight: 600;
  color: #e0def4;
  margin-bottom: 4px;
}

.template-description {
  color: #908caa;
  font-size: 13px;
}

.hierarchy-actions {
  padding: 12px 20px;
  display: flex;
  gap: 8px;
  background: rgba(20, 22, 30, 0.2);
  border-bottom: 1px solid rgba(138, 127, 219, 0.1);
}

.btn-small {
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
  border: none;
  background: rgba(138, 127, 219, 0.2);
  color: #e0def4;
}

.btn-small:hover {
  background: rgba(138, 127, 219, 0.3);
}

.hierarchy-tree {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.tree-item {
  margin-bottom: 4px;
}

.tree-group {
  background: rgba(99, 102, 241, 0.05);
  border: 1px solid rgba(99, 102, 241, 0.2);
  border-radius: 8px;
  margin-bottom: 12px;
  overflow: hidden;
  transition: all 0.2s;
  position: relative;
}

/* Improved drag feedback */
.tree-group.drag-over {
  border-color: #8a7fdb;
  background: rgba(138, 127, 219, 0.15);
  box-shadow: 0 0 10px rgba(138, 127, 219, 0.3);
}

.tree-group.drag-over-before::before,
.tree-group.drag-over-after::after,
.tree-entry.drag-over-before::before,
.tree-entry.drag-over-after::after {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  height: 3px;
  background: #8a7fdb;
  z-index: 1;
  box-shadow: 0 0 6px rgba(138, 127, 219, 0.5);
}

.tree-group.drag-over-before::before,
.tree-entry.drag-over-before::before {
  top: -2px;
}

.tree-group.drag-over-after::after,
.tree-entry.drag-over-after::after {
  bottom: -2px;
}

.group-header {
  padding: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(20, 22, 30, 0.2);
  transition: all 0.2s;
}

.group-header:hover {
  background: rgba(20, 22, 30, 0.4);
}

.drag-handle {
  color: #908caa;
  cursor: move;
  user-select: none;
  padding: 2px;
}

.drag-handle:hover {
  color: #e0def4;
}

.group-toggle {
  color: #908caa;
  cursor: pointer;
  transition: transform 0.2s;
  user-select: none;
}

.group-toggle.collapsed {
  transform: rotate(-90deg);
}

.group-name {
  flex: 1;
  color: #e0def4;
  font-size: 13px;
  font-weight: 500;
}

.group-actions {
  display: flex;
  gap: 4px;
}

.btn-icon {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: #908caa;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.2s;
}

.btn-icon:hover {
  background: rgba(138, 127, 219, 0.2);
  color: #e0def4;
}

.group-entries {
  padding: 8px;
}

.tree-entry {
  padding: 8px 12px;
  margin: 4px 0;
  background: rgba(20, 22, 30, 0.3);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 6px;
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
  position: relative;
}

.tree-entry:hover {
  background: rgba(20, 22, 30, 0.5);
  border-color: rgba(138, 127, 219, 0.2);
}

.tree-entry.active {
  background: rgba(138, 127, 219, 0.2);
  border-color: rgba(138, 127, 219, 0.4);
}

.entry-role {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
}

.role-system {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.role-assistant {
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
}

.role-user {
  background: rgba(34, 197, 94, 0.2);
  color: #4ade80;
}

.entry-name {
  flex: 1;
  color: #e0def4;
  font-size: 13px;
}

.entry-toggle {
  width: 32px;
  height: 18px;
  background: rgba(138, 127, 219, 0.2);
  border-radius: 9px;
  position: relative;
  cursor: pointer;
  transition: background 0.2s;
}

.entry-toggle.enabled {
  background: rgba(34, 197, 94, 0.4);
}

.entry-toggle::after {
  content: '';
  position: absolute;
  width: 14px;
  height: 14px;
  background: #e0def4;
  border-radius: 50%;
  top: 2px;
  left: 2px;
  transition: transform 0.2s;
}

.entry-toggle.enabled::after {
  transform: translateX(14px);
}

.content-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: rgba(20, 22, 30, 0.2);
}

.editor-header {
  padding: 20.5px;
  border-bottom: 1px solid rgba(138, 127, 219, 0.1);
  background: rgba(20, 22, 30, 0.4);
}

.editor-title {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.editor-entry-name {
  font-size: 16px;
  font-weight: 600;
  color: #e0def4;
}

.editor-meta {
  display: flex;
  gap: 16px;
  color: #908caa;
  font-size: 12px;
}

.editor-content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}

.code-textarea {
  width: 100%;
  min-height: 400px;
  background: rgba(10, 10, 14, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.1);
  border-radius: 8px;
  padding: 16px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 14px;
  line-height: 1.6;
  color: #e0def4;
  outline: none;
  resize: vertical;
}

.code-textarea:focus {
  border-color: rgba(138, 127, 219, 0.3);
}

.editor-actions {
  padding: 16px 20px;
  background: rgba(20, 22, 30, 0.4);
  border-top: 1px solid rgba(138, 127, 219, 0.1);
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

.btn-primary {
  background: linear-gradient(135deg, #8a7fdb 0%, #6366f1 100%);
  color: white;
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.btn-primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(138, 127, 219, 0.3);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.btn-secondary {
  background: rgba(138, 127, 219, 0.2);
  border: 1px solid rgba(138, 127, 219, 0.3);
  color: #e0def4;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.btn-secondary:hover {
  background: rgba(138, 127, 219, 0.3);
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #908caa;
  padding: 40px;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-text {
  font-size: 16px;
  margin-bottom: 8px;
}

.empty-subtext {
  font-size: 13px;
  text-align: center;
  max-width: 300px;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.modal {
  background: rgba(30, 33, 43, 0.98);
  border: 1px solid rgba(138, 127, 219, 0.2);
  border-radius: 12px;
  padding: 24px;
  min-width: 400px;
  max-width: 90%;
  max-height: 90vh;
  overflow-y: auto;
}

.modal-title {
  font-size: 18px;
  font-weight: 600;
  color: #e0def4;
  margin-bottom: 16px;
}

.form-group {
  margin-bottom: 16px;
}

.form-label {
  display: block;
  color: #908caa;
  font-size: 12px;
  margin-bottom: 6px;
}

.form-input {
  width: 100%;
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.2);
  color: #e0def4;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  outline: none;
}

.form-input:focus {
  border-color: rgba(138, 127, 219, 0.4);
}

.form-select {
  width: 100%;
  background: rgba(20, 22, 30, 0.6);
  border: 1px solid rgba(138, 127, 219, 0.2);
  color: #e0def4;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  outline: none;
  cursor: pointer;
}

.modal-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  margin-top: 24px;
}

::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: rgba(20, 22, 30, 0.4);
}

::-webkit-scrollbar-thumb {
  background: rgba(138, 127, 219, 0.2);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(138, 127, 219, 0.3);
}

/* Improved root drop zone */
.root-drop-zone {
  padding: 16px;
  border: 2px dashed rgba(138, 127, 219, 0.4);
  border-radius: 8px;
  text-align: center;
  color: #908caa;
  margin: 12px 0;
  transition: all 0.2s;
  background: rgba(20, 22, 30, 0.3);
}

.root-drop-zone.drag-over {
  border-color: #8a7fdb;
  background: rgba(138, 127, 219, 0.15);
  color: #e0def4;
  transform: scale(1.02);
}

/* Enhanced drag states */
body.dragging-entry .tree-group .group-header {
  background: rgba(138, 127, 219, 0.1);
}

body.dragging-group .tree-group:not(.drag-over) {
  opacity: 0.6;
}

body.dragging-entry .tree-entry:not(.drag-over-before):not(.drag-over-after) {
  opacity: 0.7;
}
</style>
