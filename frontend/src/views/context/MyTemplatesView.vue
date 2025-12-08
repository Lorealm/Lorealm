<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useTemplatesListStore } from '@/stores/templatesList'
import { useAppStateStore } from '@/stores/appState'
import type { TemplateSummary } from '@/types/context'

// ==================== Props & Emits ====================

const emit = defineEmits<{
  (e: 'edit', templateId: string): void
}>()

// ==================== Stores ====================

const templatesStore = useTemplatesListStore()
const appState = useAppStateStore()

// ==================== Local State ====================

const showCreateModal = ref(false)
const showDeleteModal = ref(false)
const templateToDelete = ref<TemplateSummary | null>(null)

const newTemplateForm = ref({
  name: '',
  description: '',
})

// ==================== Computed ====================

const templates = computed(() => templatesStore.sortedTemplates)
const isLoading = computed(() => templatesStore.isLoading)
const error = computed(() => templatesStore.error)

// ==================== Lifecycle ====================

onMounted(async () => {
  if (!templatesStore.hasLoaded) {
    await templatesStore.fetchTemplates()
  }
})

// ==================== Actions ====================

function openCreateModal(): void {
  newTemplateForm.value = { name: '', description: '' }
  showCreateModal.value = true
}

function closeCreateModal(): void {
  showCreateModal.value = false
}

async function createTemplate(): Promise<void> {
  if (!newTemplateForm.value.name.trim()) return

  const templateId = await templatesStore.createTemplate(
    newTemplateForm.value.name.trim(),
    newTemplateForm.value.description.trim()
  )

  if (templateId) {
    closeCreateModal()
    // Automatically open the new template for editing
    emit('edit', templateId)
  }
}

function editTemplate(template: TemplateSummary): void {
  emit('edit', template.id)
}

function confirmDeleteTemplate(template: TemplateSummary): void {
  templateToDelete.value = template
  showDeleteModal.value = true
}

function closeDeleteModal(): void {
  showDeleteModal.value = false
  templateToDelete.value = null
}

async function deleteTemplate(): Promise<void> {
  if (!templateToDelete.value) return

  const success = await templatesStore.deleteTemplate(templateToDelete.value.id)
  if (success) {
    closeDeleteModal()
  }
}

async function refreshList(): Promise<void> {
  await templatesStore.fetchTemplates()
}

// ==================== Formatters ====================

function formatDate(dateString: string): string {
  const date = new Date(dateString)
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins} min${diffMins > 1 ? 's' : ''} ago`
  if (diffHours < 24) return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`
  if (diffDays < 7) return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`

  return formatDate(dateString)
}
</script>

<template>
  <div class="templates-list-view">
    <!-- Header -->
    <div class="list-header">
      <div class="header-title">
        <h2>My Templates</h2>
        <span class="template-count"
          >{{ templates.length }} template{{ templates.length !== 1 ? 's' : '' }}</span
        >
      </div>
      <div class="header-actions">
        <button class="btn-icon-text" @click="refreshList" :disabled="isLoading">
          <span class="icon">🔄</span>
          Refresh
        </button>
        <button class="btn-primary" @click="openCreateModal">
          <span class="icon">+</span>
          New Template
        </button>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="isLoading && !templatesStore.hasLoaded" class="loading-state">
      <div class="spinner"></div>
      <span>Loading templates...</span>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠️</span>
      <span>{{ error }}</span>
      <button class="btn-secondary" @click="refreshList">Try Again</button>
    </div>

    <!-- Empty State -->
    <div v-else-if="templates.length === 0" class="empty-state">
      <div class="empty-icon">📝</div>
      <h3>No templates yet</h3>
      <p>Create your first template to get started with context management.</p>
      <button class="btn-primary" @click="openCreateModal">Create Template</button>
    </div>

    <!-- Templates Grid -->
    <div v-else class="templates-grid">
      <div
        v-for="template in templates"
        :key="template.id"
        class="template-card"
        :class="{ active: appState.isActiveTemplate(template.id) }"
      >
        <div class="card-header">
          <h3 class="template-name">{{ template.name }}</h3>
          <span v-if="appState.isActiveTemplate(template.id)" class="active-badge"> Active </span>
        </div>

        <p class="template-description">
          {{ template.description || 'No description' }}
        </p>

        <div class="template-meta">
          <span class="meta-item">
            <span class="meta-icon">📄</span>
            {{ template.node_count }} node{{ template.node_count !== 1 ? 's' : '' }}
          </span>
          <span class="meta-item" :title="formatDate(template.updated_at)">
            <span class="meta-icon">🕐</span>
            {{ formatRelativeTime(template.updated_at) }}
          </span>
        </div>

        <div class="card-actions">
          <button class="btn-primary" @click="editTemplate(template)">Edit</button>
          <button class="btn-danger-outline" @click="confirmDeleteTemplate(template)">
            Delete
          </button>
        </div>
      </div>
    </div>

    <!-- Create Template Modal -->
    <Teleport to="body">
      <div v-if="showCreateModal" class="modal-overlay" @click.self="closeCreateModal">
        <div class="modal">
          <div class="modal-header">
            <h3>Create New Template</h3>
          </div>

          <div class="modal-body">
            <div class="form-group">
              <label class="form-label">Template Name</label>
              <input
                class="form-input"
                v-model="newTemplateForm.name"
                placeholder="e.g., My Chat Template"
                @keyup.enter="createTemplate"
                autofocus
              />
            </div>

            <div class="form-group">
              <label class="form-label">Description (optional)</label>
              <textarea
                class="form-textarea"
                v-model="newTemplateForm.description"
                placeholder="Brief description of what this template is for..."
                rows="3"
              ></textarea>
            </div>
          </div>

          <div class="modal-actions">
            <button class="btn-secondary" @click="closeCreateModal">Cancel</button>
            <button
              class="btn-primary"
              @click="createTemplate"
              :disabled="!newTemplateForm.name.trim() || isLoading"
            >
              {{ isLoading ? 'Creating...' : 'Create Template' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Delete Confirmation Modal -->
    <Teleport to="body">
      <div v-if="showDeleteModal" class="modal-overlay" @click.self="closeDeleteModal">
        <div class="modal modal-danger">
          <div class="modal-header">
            <h3>Delete Template</h3>
          </div>

          <div class="modal-body">
            <p>
              Are you sure you want to delete <strong>{{ templateToDelete?.name }}</strong
              >?
            </p>
            <p class="warning-text">
              This action cannot be undone. All nodes in this template will be permanently deleted.
            </p>
          </div>

          <div class="modal-actions">
            <button class="btn-secondary" @click="closeDeleteModal">Cancel</button>
            <button class="btn-danger" @click="deleteTemplate" :disabled="isLoading">
              {{ isLoading ? 'Deleting...' : 'Delete Template' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.templates-list-view {
  padding: var(--spacing-xl);
  height: 100%;
  overflow-y: auto;
}

/* ============================================================================
   Header
   ============================================================================ */

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-xl);
}

.header-title {
  display: flex;
  align-items: baseline;
  gap: var(--spacing-md);
}

.header-title h2 {
  margin: 0;
  font-size: var(--font-size-2xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.template-count {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.header-actions {
  display: flex;
  gap: var(--spacing-md);
}

/* ============================================================================
   States
   ============================================================================ */

.loading-state,
.error-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-3xl);
  text-align: center;
  color: var(--color-text-secondary);
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--color-border-subtle);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: var(--spacing-md);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.error-state {
  color: var(--color-error);
}

.error-icon {
  font-size: 32px;
  margin-bottom: var(--spacing-md);
}

.error-state .btn-secondary {
  margin-top: var(--spacing-md);
}

.empty-icon {
  font-size: 48px;
  margin-bottom: var(--spacing-lg);
  opacity: 0.5;
}

.empty-state h3 {
  margin: 0 0 var(--spacing-sm) 0;
  font-size: var(--font-size-xl);
  color: var(--color-text-primary);
}

.empty-state p {
  margin: 0 0 var(--spacing-lg) 0;
  max-width: 300px;
}

/* ============================================================================
   Templates Grid
   ============================================================================ */

.templates-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--spacing-lg);
}

.template-card {
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  transition: all var(--transition-fast);
}

.template-card:hover {
  border-color: var(--color-border-medium);
  box-shadow: var(--shadow-md);
}

.template-card.active {
  border-color: var(--color-primary);
  background: var(--color-active);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--spacing-sm);
}

.template-name {
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.active-badge {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-medium);
  padding: 2px 8px;
  background: var(--color-primary);
  color: white;
  border-radius: var(--radius-full);
}

.template-description {
  margin: 0 0 var(--spacing-md) 0;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.template-meta {
  display: flex;
  gap: var(--spacing-lg);
  margin-bottom: var(--spacing-lg);
}

.meta-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.meta-icon {
  font-size: var(--font-size-base);
}

.card-actions {
  display: flex;
  gap: var(--spacing-sm);
}

/* ============================================================================
   Buttons
   ============================================================================ */

.btn-primary {
  padding: var(--spacing-sm) var(--spacing-lg);
  background: var(--color-primary);
  border: none;
  border-radius: var(--radius-md);
  color: white;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-dark-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  padding: var(--spacing-sm) var(--spacing-lg);
  background: transparent;
  border: 1px solid var(--color-border-medium);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--color-hover);
  border-color: var(--color-border-strong);
}

.btn-icon-text {
  padding: var(--spacing-sm) var(--spacing-md);
  background: transparent;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.btn-icon-text:hover:not(:disabled) {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.btn-danger-outline {
  padding: var(--spacing-sm) var(--spacing-lg);
  background: transparent;
  border: 1px solid var(--color-error);
  border-radius: var(--radius-md);
  color: var(--color-error);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-danger-outline:hover {
  background: var(--color-error-bg);
}

.btn-danger {
  padding: var(--spacing-sm) var(--spacing-lg);
  background: var(--color-error);
  border: none;
  border-radius: var(--radius-md);
  color: white;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-danger:hover:not(:disabled) {
  background: var(--color-error-dark, #c0392b);
}

.btn-danger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* ============================================================================
   Modal
   ============================================================================ */

.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--color-overlay-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-index-modal);
  backdrop-filter: var(--backdrop-blur-sm);
}

.modal {
  background: var(--color-bg-modal);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-xl);
  width: 100%;
  max-width: 440px;
  box-shadow: var(--shadow-lg);
}

.modal-danger .modal-header h3 {
  color: var(--color-error);
}

.modal-header {
  padding: var(--spacing-xl) var(--spacing-xl) var(--spacing-md);
}

.modal-header h3 {
  margin: 0;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.modal-body {
  padding: 0 var(--spacing-xl) var(--spacing-xl);
}

.modal-body p {
  margin: 0 0 var(--spacing-md) 0;
  color: var(--color-text-primary);
}

.warning-text {
  font-size: var(--font-size-sm);
  color: var(--color-error);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  padding: var(--spacing-lg) var(--spacing-xl);
  border-top: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  border-radius: 0 0 var(--radius-xl) var(--radius-xl);
}

/* ============================================================================
   Form
   ============================================================================ */

.form-group {
  margin-bottom: var(--spacing-lg);
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-label {
  display: block;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  margin-bottom: var(--spacing-sm);
  color: var(--color-text-primary);
}

.form-input,
.form-textarea {
  width: 100%;
  background: var(--color-bg-input);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
  font-family: inherit;
}

.form-input:focus,
.form-textarea:focus {
  outline: none;
  border-color: var(--color-primary);
}

.form-textarea {
  resize: vertical;
  min-height: 80px;
}

.icon {
  font-size: var(--font-size-base);
}
</style>
