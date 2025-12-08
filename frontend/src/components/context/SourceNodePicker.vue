<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { SourceDefinition, SourceNodeDefinition, SettingDefinition } from '@/types/context'

// ==================== Props & Emits ====================

const props = defineProps<{
  show: boolean
  availableSources: SourceDefinition[]
  usedSourceNodes: [string, string][] // [source_id, node_id][]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'select', payload: { 
    sourceId: string
    nodeId: string 
    title: string
    settings: Record<string, unknown>
  }): void
}>()

// ==================== Local State ====================

type Step = 'select-source' | 'select-node' | 'configure'

const currentStep = ref<Step>('select-source')
const selectedSource = ref<SourceDefinition | null>(null)
const selectedNode = ref<SourceNodeDefinition | null>(null)
const settings = ref<Record<string, unknown>>({})
const customTitle = ref('')

// ==================== Computed ====================

// Filter out already-used source nodes
const availableNodes = computed(() => {
  if (!selectedSource.value) return []
  
  const usedSet = new Set(
    props.usedSourceNodes
      .filter(([sId]) => sId === selectedSource.value?.source_id)
      .map(([_, nId]) => nId)
  )
  
  return selectedSource.value.nodes.filter(node => !usedSet.has(node.node_id))
})

const canProceedToNode = computed(() => selectedSource.value !== null)
const canProceedToConfig = computed(() => selectedNode.value !== null)
const canSubmit = computed(() => {
  if (!selectedNode.value) return false
  
  // Check required settings
  for (const setting of selectedNode.value.settings) {
    if (setting.required && !settings.value[setting.key]) {
      return false
    }
  }
  
  return true
})

// ==================== Watchers ====================

// Reset state when modal opens/closes
watch(() => props.show, (show) => {
  if (show) {
    resetState()
  }
})

// Reset node selection when source changes
watch(selectedSource, () => {
  selectedNode.value = null
  settings.value = {}
})

// Initialize settings with defaults when node is selected
watch(selectedNode, (node) => {
  if (node) {
    const defaults: Record<string, unknown> = {}
    for (const setting of node.settings) {
      if (setting.default_value !== undefined && setting.default_value !== null) {
        defaults[setting.key] = setting.default_value
      } else {
        // Set sensible defaults based on setting_type
        const settingType = setting.setting_type.type
        switch (settingType) {
          case 'boolean':
            defaults[setting.key] = false
            break
          case 'integer':
          case 'token_count':
            defaults[setting.key] = (setting.setting_type as { min?: number }).min ?? 0
            break
          case 'float':
            defaults[setting.key] = (setting.setting_type as { min?: number }).min ?? 0.0
            break
          case 'string':
            defaults[setting.key] = ''
            break
          case 'select': {
            const options = (setting.setting_type as { options: { value: string }[] }).options
            defaults[setting.key] = options?.[0]?.value ?? ''
            break
          }
        }
      }
    }
    settings.value = defaults
    customTitle.value = node.title
  }
})

// ==================== Methods ====================

function resetState(): void {
  currentStep.value = 'select-source'
  selectedSource.value = null
  selectedNode.value = null
  settings.value = {}
  customTitle.value = ''
}

function close(): void {
  emit('close')
}

function selectSource(source: SourceDefinition): void {
  selectedSource.value = source
  currentStep.value = 'select-node'
}

function selectNode(node: SourceNode): void {
  selectedNode.value = node
  currentStep.value = 'configure'
}

function goBack(): void {
  if (currentStep.value === 'configure') {
    currentStep.value = 'select-node'
  } else if (currentStep.value === 'select-node') {
    currentStep.value = 'select-source'
  }
}

function submit(): void {
  if (!selectedSource.value || !selectedNode.value) return
  
  const payload = {
    sourceId: selectedSource.value.source_id,
    nodeId: selectedNode.value.node_id,
    title: customTitle.value || selectedNode.value.title,
    settings: settings.value,
  }
  
  // Debug logging
  console.log('SourceNodePicker submit - selectedSource:', selectedSource.value)
  console.log('SourceNodePicker submit - selectedNode:', selectedNode.value)
  console.log('SourceNodePicker submit - payload:', payload)
  
  emit('select', payload)
}

// ==================== Icon Mapping ====================

function getSourceIcon(sourceId: string): string {
  const icons: Record<string, string> = {
    'chat': '💬',
    'character': '👤',
    'user': '🧑',
    'lorebook': '📚',
    'system': '⚙️',
    'memory': '🧠',
    'context': '📝',
  }
  return icons[sourceId] || '📦'
}
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="modal-overlay" @click.self="close">
      <div class="modal source-picker-modal">
        <!-- Header -->
        <div class="modal-header">
          <button 
            v-if="currentStep !== 'select-source'" 
            class="back-button" 
            @click="goBack"
          >
            ← Back
          </button>
          <h3>
            <template v-if="currentStep === 'select-source'">Select Source</template>
            <template v-else-if="currentStep === 'select-node'">Select Node</template>
            <template v-else>Configure Settings</template>
          </h3>
          <button class="close-button" @click="close">×</button>
        </div>
        
        <!-- Step 1: Select Source -->
        <div v-if="currentStep === 'select-source'" class="modal-body">
          <p class="step-description">
            Choose a data source to add to your template.
          </p>
          
          <div class="source-list">
            <button
              v-for="source in availableSources"
              :key="source.source_id"
              class="source-item"
              @click="selectSource(source)"
            >
              <span class="source-icon">{{ getSourceIcon(source.source_id) }}</span>
              <div class="source-info">
                <span class="source-name">{{ source.name }}</span>
                <span class="source-description">{{ source.description }}</span>
              </div>
              <span class="source-node-count">
                {{ source.nodes.length }} node{{ source.nodes.length !== 1 ? 's' : '' }}
              </span>
            </button>
          </div>
        </div>
        
        <!-- Step 2: Select Node -->
        <div v-else-if="currentStep === 'select-node'" class="modal-body">
          <p class="step-description">
            Choose a node from <strong>{{ selectedSource?.name }}</strong>.
          </p>
          
          <div v-if="availableNodes.length === 0" class="empty-state">
            <span class="empty-icon">📭</span>
            <p>All nodes from this source are already in use.</p>
          </div>
          
          <div v-else class="node-list">
            <button
              v-for="node in availableNodes"
              :key="node.node_id"
              class="node-item"
              @click="selectNode(node)"
            >
              <div class="node-info">
                <span class="node-name">{{ node.title }}</span>
                <span class="node-description">{{ node.description }}</span>
              </div>
              <span v-if="node.settings.length > 0" class="node-settings-count">
                {{ node.settings.length }} setting{{ node.settings.length !== 1 ? 's' : '' }}
              </span>
            </button>
          </div>
        </div>
        
        <!-- Step 3: Configure Settings -->
        <div v-else-if="currentStep === 'configure'" class="modal-body">
          <p class="step-description">
            Configure <strong>{{ selectedNode?.title }}</strong> settings.
          </p>
          
          <div class="settings-form">
            <!-- Custom title -->
            <div class="form-group">
              <label class="form-label">Display Name</label>
              <input 
                class="form-input"
                v-model="customTitle"
                :placeholder="selectedNode?.title"
              />
              <span class="form-hint">The name shown in your template tree.</span>
            </div>
            
            <!-- Dynamic settings -->
            <template v-if="selectedNode">
              <div 
                v-for="setting in selectedNode.settings" 
                :key="setting.key"
                class="form-group"
              >
                <label class="form-label">
                  {{ setting.label }}
                  <span v-if="setting.required" class="required-mark">*</span>
                </label>
                
                <!-- String input -->
                <input 
                  v-if="setting.setting_type.type === 'string'"
                  class="form-input"
                  v-model="settings[setting.key]"
                  :placeholder="setting.description"
                  :maxlength="(setting.setting_type as any).max_length"
                />
                
                <!-- Integer input -->
                <input 
                  v-else-if="setting.setting_type.type === 'integer' || setting.setting_type.type === 'token_count'"
                  type="number"
                  class="form-input"
                  v-model.number="settings[setting.key]"
                  :min="(setting.setting_type as any).min"
                  :max="(setting.setting_type as any).max"
                />
                
                <!-- Float input -->
                <input 
                  v-else-if="setting.setting_type.type === 'float'"
                  type="number"
                  step="0.1"
                  class="form-input"
                  v-model.number="settings[setting.key]"
                  :min="(setting.setting_type as any).min"
                  :max="(setting.setting_type as any).max"
                />
                
                <!-- Boolean toggle -->
                <div v-else-if="setting.setting_type.type === 'boolean'" class="toggle-wrapper">
                  <button 
                    class="toggle-button"
                    :class="{ active: settings[setting.key] }"
                    @click="settings[setting.key] = !settings[setting.key]"
                  >
                    <span class="toggle-track">
                      <span class="toggle-thumb"></span>
                    </span>
                    <span class="toggle-label">
                      {{ settings[setting.key] ? 'Enabled' : 'Disabled' }}
                    </span>
                  </button>
                </div>
                
                <!-- Select dropdown -->
                <select 
                  v-else-if="setting.setting_type.type === 'select'"
                  class="form-select"
                  v-model="settings[setting.key]"
                >
                  <option 
                    v-for="option in (setting.setting_type as any).options" 
                    :key="option.value" 
                    :value="option.value"
                  >
                    {{ option.label }}
                  </option>
                </select>
                
                <span v-if="setting.description" class="form-hint">
                  {{ setting.description }}
                </span>
              </div>
            </template>
          </div>
        </div>
        
        <!-- Footer -->
        <div class="modal-actions">
          <button class="btn-secondary" @click="close">Cancel</button>
          <button 
            v-if="currentStep === 'configure'"
            class="btn-primary"
            :disabled="!canSubmit"
            @click="submit"
          >
            Add Source Node
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
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

.source-picker-modal {
  background: var(--color-bg-modal);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-xl);
  width: 100%;
  max-width: 500px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
}

/* ============================================================================
   Header
   ============================================================================ */

.modal-header {
  display: flex;
  align-items: center;
  padding: var(--spacing-lg) var(--spacing-xl);
  border-bottom: 1px solid var(--color-border-subtle);
  gap: var(--spacing-md);
}

.modal-header h3 {
  flex: 1;
  margin: 0;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.back-button {
  background: none;
  border: none;
  color: var(--color-text-secondary);
  cursor: pointer;
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--font-size-sm);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.back-button:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.close-button {
  background: none;
  border: none;
  font-size: var(--font-size-xl);
  color: var(--color-text-secondary);
  cursor: pointer;
  padding: var(--spacing-xs);
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.close-button:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

/* ============================================================================
   Body
   ============================================================================ */

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-xl);
}

.step-description {
  margin: 0 0 var(--spacing-lg) 0;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

/* ============================================================================
   Source List
   ============================================================================ */

.source-list,
.node-list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.source-item,
.node-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  padding: var(--spacing-md) var(--spacing-lg);
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  cursor: pointer;
  text-align: left;
  transition: all var(--transition-fast);
}

.source-item:hover,
.node-item:hover {
  background: var(--color-hover);
  border-color: var(--color-border-medium);
}

.source-icon {
  font-size: 24px;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-overlay);
  border-radius: var(--radius-md);
}

.source-info,
.node-info {
  flex: 1;
  min-width: 0;
}

.source-name,
.node-name {
  display: block;
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
  margin-bottom: 2px;
}

.source-description,
.node-description {
  display: block;
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.source-node-count,
.node-settings-count {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  background: var(--color-bg-overlay);
  padding: 2px 8px;
  border-radius: var(--radius-full);
}

/* ============================================================================
   Empty State
   ============================================================================ */

.empty-state {
  text-align: center;
  padding: var(--spacing-2xl);
  color: var(--color-text-secondary);
}

.empty-icon {
  font-size: 32px;
  display: block;
  margin-bottom: var(--spacing-md);
  opacity: 0.5;
}

/* ============================================================================
   Settings Form
   ============================================================================ */

.settings-form {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.form-label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  color: var(--color-text-primary);
}

.required-mark {
  color: var(--color-error);
}

.form-input,
.form-select {
  width: 100%;
  background: var(--color-bg-input);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  padding: var(--spacing-sm) var(--spacing-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
}

.form-input:focus,
.form-select:focus {
  outline: none;
  border-color: var(--color-primary);
}

.form-hint {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

/* Toggle */
.toggle-wrapper {
  display: flex;
}

.toggle-button {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
}

.toggle-track {
  width: 36px;
  height: 20px;
  background: var(--color-bg-overlay);
  border-radius: var(--radius-full);
  position: relative;
  transition: background var(--transition-fast);
}

.toggle-button.active .toggle-track {
  background: var(--color-primary);
}

.toggle-thumb {
  position: absolute;
  width: 16px;
  height: 16px;
  background: white;
  border-radius: 50%;
  top: 2px;
  left: 2px;
  transition: transform var(--transition-fast);
}

.toggle-button.active .toggle-thumb {
  transform: translateX(16px);
}

.toggle-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

/* ============================================================================
   Actions
   ============================================================================ */

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  padding: var(--spacing-lg) var(--spacing-xl);
  border-top: 1px solid var(--color-border-subtle);
}

.btn-primary {
  padding: var(--spacing-sm) var(--spacing-xl);
  background: var(--color-primary);
  border: none;
  border-radius: var(--radius-md);
  color: white;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-dark-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  padding: var(--spacing-sm) var(--spacing-xl);
  background: transparent;
  border: 1px solid var(--color-border-medium);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-secondary:hover {
  background: var(--color-hover);
}
</style>