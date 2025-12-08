<script setup lang="ts">
import { ref, reactive, computed, provide, watch, onMounted, onBeforeUnmount } from 'vue'
import { useTemplateStore } from '@/stores/template'
import { useAppStateStore } from '@/stores/appState'
import TreeNodeList from './TreeNodeList.vue'
import SaveIndicator from '@/components/context/SaveIndicator.vue'
import SourceNodePicker from '@/components/context/SourceNodePicker.vue'
import type {
  TreeNode,
  Group,
  Entry,
  GroupModalState,
  EntryModalState,
  ContextEditorContext,
  NodePath,
} from '@/types/context'
import { isGroup, isEntry } from '@/types/context'

// ==================== Props ====================

const props = defineProps<{
  templateId?: string
}>()

const emit = defineEmits<{
  (e: 'back'): void
}>()

// ==================== Stores ====================

const templateStore = useTemplateStore()
const appState = useAppStateStore()

// ==================== Constants ====================

const MAX_NESTING_DEPTH = 3

// ==================== Local UI State ====================

const selectedEntry = ref<Entry | null>(null)
const contentChanged = ref(false)
const collapsedGroups = ref<Set<string>>(new Set())
const showSourcePicker = ref(false)
const sourcePickerParentPath = ref<NodePath | undefined>(undefined)

// Modal states
const groupModal = reactive<GroupModalState>({
  show: false,
  mode: 'create',
  parentGroupId: null,
  editingId: null,
  form: {
    name: '',
    description: '',
    color: '',
  },
})

const entryModal = reactive<EntryModalState>({
  show: false,
  mode: 'create',
  parentGroupId: null,
  editingId: null,
  form: {
    name: '',
    role: 'System',
    groupId: '',
  },
})

// ==================== Computed ====================

const treeNodes = computed({
  get: () => templateStore.treeNodes,
  set: (value) => {
    // Tree updates are handled by individual node operations
    // This setter is for vuedraggable compatibility
  }
})

const allGroupsForDropdown = computed(() => getAllGroups())
const indentString = (depth: number): string => '\u00A0\u00A0'.repeat(depth)

// ==================== Lifecycle ====================

onMounted(async () => {
  // Determine which template to load
  const idToLoad = props.templateId || appState.activeTemplateId
  
  if (idToLoad) {
    await templateStore.loadTemplate(idToLoad)
  }
})

// Warn about unsaved changes when leaving
onBeforeUnmount(() => {
  if (templateStore.hasUnsavedChanges) {
    // Could show a confirmation dialog here
    console.warn('Leaving with unsaved changes')
  }
})

// Browser beforeunload warning
function handleBeforeUnload(e: BeforeUnloadEvent): void {
  if (templateStore.hasUnsavedChanges) {
    e.preventDefault()
    e.returnValue = ''
  }
}

onMounted(() => {
  window.addEventListener('beforeunload', handleBeforeUnload)
})

onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', handleBeforeUnload)
})

// ==================== Utility Functions ====================

function generateId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

function findGroupById(id: string, nodes: TreeNode[] = templateStore.treeNodes): Group | null {
  for (const node of nodes) {
    if (isGroup(node)) {
      if (node.data.id === id) return node.data
      const found = findGroupById(id, node.data.children)
      if (found) return found
    }
  }
  return null
}

function findNodeById(id: string, nodes: TreeNode[] = templateStore.treeNodes): TreeNode | null {
  for (const node of nodes) {
    if (node.data.id === id) return node
    if (isGroup(node)) {
      const found = findNodeById(id, node.data.children)
      if (found) return found
    }
  }
  return null
}

function findParentList(nodeId: string, nodes: TreeNode[] = templateStore.treeNodes): TreeNode[] | null {
  for (const node of nodes) {
    if (node.data.id === nodeId) return nodes
    if (isGroup(node)) {
      const found = findParentList(nodeId, node.data.children)
      if (found) return found
    }
  }
  return null
}

function getAllGroups(nodes: TreeNode[] = templateStore.treeNodes, result: Group[] = []): Group[] {
  for (const node of nodes) {
    if (isGroup(node)) {
      result.push(node.data)
      getAllGroups(node.data.children, result)
    }
  }
  return result
}

function getGroupDepth(groupId: string): number {
  const group = findGroupById(groupId)
  return group?.depth ?? -1
}

// ==================== UI Actions ====================

function selectEntry(entry: Entry): void {
  selectedEntry.value = entry
  contentChanged.value = false
}

function toggleGroup(groupId: string): void {
  if (collapsedGroups.value.has(groupId)) {
    collapsedGroups.value.delete(groupId)
  } else {
    collapsedGroups.value.add(groupId)
  }
}

function toggleEntry(entry: Entry): void {
  if (entry.locked) return
  templateStore.toggleNode(entry.id)
}

function goBack(): void {
  emit('back')
}

// ==================== Content Editing ====================

function onContentInput(): void {
  if (!selectedEntry.value || selectedEntry.value.locked) return
  contentChanged.value = true
  templateStore.updateEntryContent(selectedEntry.value, selectedEntry.value.content)
}

function saveContent(): void {
  contentChanged.value = false
}

function revertChanges(): void {
  // Reload the template to get fresh data
  if (templateStore.templateId) {
    templateStore.reloadTemplate()
  }
  contentChanged.value = false
}

// ==================== Group CRUD ====================

function openAddGroupModal(parentGroupId: string | null = null): void {
  groupModal.show = true
  groupModal.mode = 'create'
  groupModal.parentGroupId = parentGroupId
  groupModal.editingId = null
  groupModal.form.name = ''
  groupModal.form.description = ''
  groupModal.form.color = ''
}

function openEditGroupModal(group: Group): void {
  groupModal.show = true
  groupModal.mode = 'edit'
  groupModal.parentGroupId = null
  groupModal.editingId = group.id
  groupModal.form.name = group.name
  groupModal.form.description = group.description || ''
  groupModal.form.color = group.color || ''
}

function closeGroupModal(): void {
  groupModal.show = false
}

async function submitGroupModal(): Promise<void> {
  if (groupModal.mode === 'create') {
    await createGroup()
  } else {
    await updateGroup()
  }
}

async function createGroup(): Promise<void> {
  const parentPath = groupModal.parentGroupId 
    ? templateStore.getNodePath(groupModal.parentGroupId) ?? undefined
    : undefined
  
  // Check depth constraint
  if (groupModal.parentGroupId) {
    const parentDepth = getGroupDepth(groupModal.parentGroupId)
    if (parentDepth >= MAX_NESTING_DEPTH - 1) {
      alert(`Cannot nest groups more than ${MAX_NESTING_DEPTH} levels deep`)
      return
    }
  }
  
  await templateStore.addGroup(
    groupModal.form.name || 'New Group',
    groupModal.form.description || undefined,
    parentPath
  )
  
  closeGroupModal()
}

async function updateGroup(): Promise<void> {
  if (!groupModal.editingId) return
  
  const group = findGroupById(groupModal.editingId)
  if (!group) return
  
  templateStore.updateGroup(
    group,
    groupModal.form.name || 'Unnamed Group',
    groupModal.form.description || undefined
  )
  
  closeGroupModal()
}

async function deleteGroup(group: Group): Promise<void> {
  if (!confirm(`Delete group "${group.name}"? Its contents will be moved to the root level.`)) {
    return
  }
  
  await templateStore.deleteNode(group.id)
}

// ==================== Entry CRUD ====================

function openAddEntryModal(parentGroupId: string | null = null): void {
  entryModal.show = true
  entryModal.mode = 'create'
  entryModal.parentGroupId = parentGroupId
  entryModal.form.name = ''
  entryModal.form.role = 'System'
  entryModal.form.groupId = parentGroupId || ''
}

function closeEntryModal(): void {
  entryModal.show = false
}

async function createEntry(): Promise<void> {
  const parentPath = entryModal.form.groupId
    ? templateStore.getNodePath(entryModal.form.groupId) ?? undefined
    : undefined
  
  await templateStore.addTextEntry(
    entryModal.form.name || 'New Entry',
    entryModal.form.role.toLowerCase() as 'system' | 'user' | 'assistant',
    '',
    parentPath
  )
  
  closeEntryModal()
}

async function deleteEntry(entry: Entry): Promise<void> {
  if (!confirm(`Delete entry "${entry.name}"?`)) {
    return
  }
  
  await templateStore.deleteNode(entry.id)
  
  if (selectedEntry.value?.id === entry.id) {
    selectedEntry.value = null
  }
}

// ==================== Source Node ====================

function openSourcePicker(parentGroupId: string | null = null): void {
  sourcePickerParentPath.value = parentGroupId 
    ? templateStore.getNodePath(parentGroupId) ?? undefined
    : undefined
  showSourcePicker.value = true
}

function closeSourcePicker(): void {
  showSourcePicker.value = false
}

async function handleSourceNodeSelect(payload: {
  sourceId: string
  nodeId: string
  title: string
  settings: Record<string, unknown>
}): Promise<void> {
  console.log('handleSourceNodeSelect received:', payload)
  
  await templateStore.addSourceNode(
    payload.sourceId,
    payload.nodeId,
    payload.title,
    payload.settings,
    sourcePickerParentPath.value
  )
  
  closeSourcePicker()
}

// ==================== Drag & Drop ====================

function checkMove(evt: any): boolean {
  const draggedEl = evt.dragged
  const toEl = evt.to
  
  const draggedId = draggedEl?.dataset?.nodeId
  const draggedType = draggedEl?.dataset?.nodeType
  
  // Entries can always be moved
  if (draggedType === 'entry') return true
  
  // For groups, check depth constraints
  if (draggedType === 'group' && draggedId) {
    const draggedGroup = findGroupById(draggedId)
    if (!draggedGroup) return true
    
    const targetGroupId = toEl?.dataset?.groupId
    let targetDepth = -1
    
    if (targetGroupId) {
      const targetGroup = findGroupById(targetGroupId)
      if (targetGroup) {
        targetDepth = targetGroup.depth
      }
    }
    
    // Calculate max depth in dragged group
    const getMaxDepth = (group: Group): number => {
      let max = 0
      for (const child of group.children) {
        if (child.type === 'group') {
          max = Math.max(max, 1 + getMaxDepth(child.data as Group))
        }
      }
      return max
    }
    
    const maxDepthInDragged = getMaxDepth(draggedGroup)
    const wouldBeDepth = targetDepth + 1 + maxDepthInDragged
    
    return wouldBeDepth <= MAX_NESTING_DEPTH
  }
  
  return true
}

// Track the original path of the dragged node
const draggedNodeOriginalPath = ref<number[] | null>(null)
// Track all group paths at the start of drag (before vuedraggable modifies the tree)
const groupPathsBeforeDrag = ref<Map<string, number[]>>(new Map())

function onDragStart(nodeId: string): void {
  // Save the original path before the drag modifies the tree
  const path = templateStore.getNodePath(nodeId)
  draggedNodeOriginalPath.value = path ? [...path] : null
  
  // Save paths of all groups before drag
  const groupPaths = new Map<string, number[]>()
  function collectGroupPaths(nodes: TreeNode[], prefix: number[] = []): void {
    nodes.forEach((node, index) => {
      const currentPath = [...prefix, index]
      if (isGroup(node)) {
        groupPaths.set(node.data.id, currentPath)
        collectGroupPaths(node.data.children, currentPath)
      }
    })
  }
  collectGroupPaths(templateStore.treeNodes)
  groupPathsBeforeDrag.value = groupPaths
  
  console.log('Drag started:', { nodeId, originalPath: draggedNodeOriginalPath.value, groupPaths: Object.fromEntries(groupPaths) })
}

async function onNodeMoved(
  nodeId: string,
  newParentGroupId: string | null,
  newIndex: number
): Promise<void> {
  console.log('Node moved:', { nodeId, newParentGroupId, newIndex, originalPath: draggedNodeOriginalPath.value })
  
  // Use the ORIGINAL path of the parent group (before drag modified the tree)
  let newParentPath: number[] | undefined = undefined
  
  if (newParentGroupId) {
    // Use the saved path from before the drag
    newParentPath = groupPathsBeforeDrag.value.get(newParentGroupId)
    console.log('Parent group path (from before drag):', newParentPath)
    
    if (!newParentPath) {
      console.error('Could not find original path for group:', newParentGroupId)
      // Reload to restore consistent state
      await templateStore.reloadTemplate()
      draggedNodeOriginalPath.value = null
      groupPathsBeforeDrag.value.clear()
      return
    }
  }
  
  // Check if anything actually changed
  const originalPath = draggedNodeOriginalPath.value
  if (originalPath) {
    const originalParentPath = originalPath.length > 1 ? originalPath.slice(0, -1) : undefined
    const originalIndex = originalPath[originalPath.length - 1]
    
    const sameParent = JSON.stringify(originalParentPath) === JSON.stringify(newParentPath)
    const sameIndex = originalIndex === newIndex
    
    if (sameParent && sameIndex) {
      console.log('No change detected, skipping API call')
      draggedNodeOriginalPath.value = null
      groupPathsBeforeDrag.value.clear()
      return
    }
  }
  
  // Call the API to persist the move
  const success = await templateStore.moveNode(nodeId, newParentPath, newIndex)
  
  if (!success) {
    console.error('Move failed, reloading template')
    // Reload to restore correct state
    await templateStore.reloadTemplate()
  }
  
  draggedNodeOriginalPath.value = null
  groupPathsBeforeDrag.value.clear()
}

// ==================== Provide Context to Children ====================

const editorContext: ContextEditorContext = {
  get selectedEntry() {
    return selectedEntry.value
  },
  collapsedGroups: collapsedGroups.value,
  maxNestingDepth: MAX_NESTING_DEPTH,

  // Drag & Drop
  checkMove,
  onDragStart,
  onNodeMoved,

  // UI Actions
  selectEntry,
  toggleGroup,
  toggleEntry,

  // CRUD
  openAddGroupModal,
  openEditGroupModal,
  deleteGroup,
  openAddEntryModal,
  deleteEntry,

  // Utilities
  findGroupById,
}

provide('contextEditor', editorContext)
</script>

<template>
  <div class="template-editor-container">
    <!-- Loading State -->
    <div v-if="templateStore.isLoading && !templateStore.isLoaded" class="loading-overlay">
      <div class="spinner"></div>
      <span>Loading template...</span>
    </div>
    
    <!-- No Template State -->
    <div v-else-if="!templateStore.isLoaded" class="empty-state-container">
      <div class="empty-state">
        <div class="empty-icon">📝</div>
        <h3>No template selected</h3>
        <p>Select a template from the list to start editing.</p>
        <button class="btn-primary" @click="goBack">
          ← Back to Templates
        </button>
      </div>
    </div>
    
    <!-- Editor -->
    <template v-else>
      <!-- ================================================================== -->
      <!-- Hierarchy Panel -->
      <!-- ================================================================== -->
      <div class="template-hierarchy">
        <div class="hierarchy-header">
          <button class="back-button" @click="goBack">← Back</button>
          <div class="template-info">
            <div class="template-title">{{ templateStore.templateName }}</div>
            <div class="template-description">{{ templateStore.templateDescription }}</div>
          </div>
          <SaveIndicator 
            :state="templateStore.saveState"
            :last-saved-at="templateStore.lastSavedAt"
            :error="templateStore.saveError"
          />
        </div>

        <div class="hierarchy-actions">
          <button class="btn-small" @click="openAddGroupModal(null)">+ Group</button>
          <button class="btn-small" @click="openAddEntryModal(null)">+ Entry</button>
          <button class="btn-small btn-source" @click="openSourcePicker(null)">+ Source</button>
        </div>

        <div class="hierarchy-tree">
          <TreeNodeList 
            v-model:nodes="treeNodes" 
            :parent-group-id="null" 
            :depth="0" 
          />
        </div>
      </div>

      <!-- ================================================================== -->
      <!-- Content Editor -->
      <!-- ================================================================== -->
      <div class="content-editor">
        <template v-if="selectedEntry">
          <div class="editor-header">
            <div class="editor-title">
              <span :class="'entry-role role-' + selectedEntry.role.toLowerCase()">
                {{ selectedEntry.role }}
              </span>
              <span class="editor-entry-name">
                {{ selectedEntry.name }}
                <span v-if="selectedEntry.locked" class="lock-badge">🔒 Read-only</span>
              </span>
            </div>
            <div class="editor-meta">
              <span>Status: {{ selectedEntry.enabled ? 'Enabled' : 'Disabled' }}</span>
              <span v-if="selectedEntry.computedSource">
                Source: {{ selectedEntry.computedSource }}
              </span>
              <span>ID: {{ selectedEntry.id.substring(0, 8) }}...</span>
            </div>
          </div>

          <div class="editor-content">
            <div v-if="selectedEntry.locked" class="locked-notice">
              <p>
                <strong>This entry is automatically generated.</strong>
                <br />
                Its content comes from: <code>{{ selectedEntry.computedSource }}</code>
                <br />
                You can move or delete this entry, but you cannot edit its content.
              </p>
              
              <!-- Show settings if it's a source node -->
              <div v-if="selectedEntry.settings && Object.keys(selectedEntry.settings).length > 0" class="source-settings">
                <strong>Settings:</strong>
                <ul>
                  <li v-for="(value, key) in selectedEntry.settings" :key="key">
                    {{ key }}: {{ value }}
                  </li>
                </ul>
              </div>
            </div>
            <textarea
              class="code-textarea"
              v-model="selectedEntry.content"
              @input="onContentInput"
              :disabled="selectedEntry.locked"
              :placeholder="
                selectedEntry.locked
                  ? 'This content is automatically generated and cannot be edited.'
                  : 'Enter your template content here...'
              "
            ></textarea>
          </div>

          <div class="editor-actions" v-if="!selectedEntry.locked">
            <button class="btn-secondary" @click="revertChanges" :disabled="!contentChanged">
              Revert
            </button>
            <button class="btn-primary" @click="saveContent" :disabled="!contentChanged">
              Save Content
            </button>
          </div>
        </template>

        <div v-else class="empty-state">
          <div class="empty-icon">📝</div>
          <div class="empty-text">No entry selected</div>
          <div class="empty-subtext">
            Select an entry from the hierarchy to edit its content, or create a new entry to get
            started.
          </div>
        </div>
      </div>
    </template>
  </div>

  <!-- ================================================================== -->
  <!-- Add/Edit Group Modal -->
  <!-- ================================================================== -->
  <Teleport to="body">
    <div v-if="groupModal.show" class="modal-overlay" @click.self="closeGroupModal">
      <div class="modal">
        <div class="modal-title">
          {{ groupModal.mode === 'edit' ? 'Edit Group' : 'Add New Group' }}
          <span
            v-if="groupModal.parentGroupId && groupModal.mode === 'create'"
            class="modal-subtitle"
          >
            Adding to: {{ findGroupById(groupModal.parentGroupId)?.name }}
          </span>
        </div>
        <div class="form-group">
          <label class="form-label">Group Name</label>
          <input
            class="form-input"
            v-model="groupModal.form.name"
            placeholder="e.g., Character Information"
            @keyup.enter="submitGroupModal"
          />
        </div>
        <div class="form-group">
          <label class="form-label">Description (Optional)</label>
          <input
            class="form-input"
            v-model="groupModal.form.description"
            placeholder="Brief description of this group"
          />
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" @click="closeGroupModal">Cancel</button>
          <button class="btn-primary" @click="submitGroupModal">
            {{ groupModal.mode === 'edit' ? 'Update Group' : 'Add Group' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- ================================================================== -->
  <!-- Add Entry Modal -->
  <!-- ================================================================== -->
  <Teleport to="body">
    <div v-if="entryModal.show" class="modal-overlay" @click.self="closeEntryModal">
      <div class="modal">
        <div class="modal-title">Add New Entry</div>
        <div class="form-group">
          <label class="form-label">Entry Name</label>
          <input
            class="form-input"
            v-model="entryModal.form.name"
            placeholder="e.g., System Instructions"
            @keyup.enter="createEntry"
          />
        </div>
        <div class="form-group">
          <label class="form-label">Role</label>
          <select class="form-select" v-model="entryModal.form.role">
            <option value="System">System</option>
            <option value="Assistant">Assistant</option>
            <option value="User">User</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">Add to Group (Optional)</label>
          <select class="form-select" v-model="entryModal.form.groupId">
            <option value="">No Group (Root Level)</option>
            <option v-for="group in allGroupsForDropdown" :key="group.id" :value="group.id">
              {{ indentString(group.depth) }}{{ group.name }}
            </option>
          </select>
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" @click="closeEntryModal">Cancel</button>
          <button class="btn-primary" @click="createEntry">Add Entry</button>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- ================================================================== -->
  <!-- Source Node Picker -->
  <!-- ================================================================== -->
  <SourceNodePicker
    :show="showSourcePicker"
    :available-sources="templateStore.availableSources"
    :used-source-nodes="templateStore.usedSourceNodes"
    @close="closeSourcePicker"
    @select="handleSourceNodeSelect"
  />
</template>

<style scoped>
/* ============================================================================
   Layout
   ============================================================================ */

.template-editor-container {
  display: flex;
  height: 100%;
  background: var(--color-bg-base);
  color: var(--color-text-primary);
  position: relative;
}

.loading-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--color-bg-base);
  z-index: 10;
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
  to { transform: rotate(360deg); }
}

.empty-state-container {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.template-hierarchy {
  width: 380px;
  min-width: 300px;
  max-width: 500px;
  border-right: 1px solid var(--color-border-subtle);
  display: flex;
  flex-direction: column;
  background: var(--color-bg-elevated);
}

.content-editor {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: var(--spacing-2xl);
  overflow: auto;
}

/* ============================================================================
   Hierarchy Header & Actions
   ============================================================================ */

.hierarchy-header {
  padding: var(--spacing-lg);
  border-bottom: 1px solid var(--color-border-subtle);
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.back-button {
  align-self: flex-start;
  background: none;
  border: none;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  cursor: pointer;
  padding: var(--spacing-xs) 0;
  transition: color var(--transition-fast);
}

.back-button:hover {
  color: var(--color-text-primary);
}

.template-info {
  flex: 1;
}

.template-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  margin-bottom: var(--spacing-xs);
  color: var(--color-text-primary);
}

.template-description {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.hierarchy-actions {
  display: flex;
  gap: var(--spacing-sm);
  padding: var(--spacing-md) var(--spacing-lg);
  border-bottom: 1px solid var(--color-border-subtle);
}

.btn-small {
  padding: var(--spacing-sm) var(--spacing-md);
  font-size: var(--font-size-sm);
  background: var(--color-bg-elevated-2);
  border: 1px solid var(--color-border-medium);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-small:hover {
  background: var(--color-hover);
  border-color: var(--color-border-strong);
}

.btn-source {
  background: rgba(139, 92, 246, 0.1);
  border-color: rgba(139, 92, 246, 0.3);
  color: #a78bfa;
}

.btn-source:hover {
  background: rgba(139, 92, 246, 0.2);
  border-color: rgba(139, 92, 246, 0.5);
}

/* ============================================================================
   Tree
   ============================================================================ */

.hierarchy-tree {
  flex: 1;
  overflow-y: auto;
  padding: var(--spacing-md);
}

/* ============================================================================
   Content Editor
   ============================================================================ */

.editor-header {
  margin-bottom: var(--spacing-xl);
}

.editor-title {
  display: flex;
  align-items: center;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-sm);
}

.entry-role {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.role-system {
  background: rgba(139, 92, 246, 0.2);
  color: #a78bfa;
}

.role-user {
  background: var(--color-info-bg);
  color: var(--color-info-text);
}

.role-assistant {
  background: var(--color-success-bg);
  color: var(--color-success-text);
}

.editor-entry-name {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.lock-badge {
  font-size: var(--font-size-sm);
  background: var(--color-warning);
  color: #000;
  padding: 3px var(--spacing-sm);
  border-radius: var(--radius-full);
  margin-left: var(--spacing-sm);
  font-weight: var(--font-weight-medium);
}

.editor-meta {
  display: flex;
  gap: var(--spacing-lg);
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.editor-content {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.locked-notice {
  background: rgba(255, 179, 0, 0.1);
  border: 1px solid var(--color-warning);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  margin-bottom: var(--spacing-lg);
}

.locked-notice p {
  margin: 0;
  font-size: var(--font-size-base);
  line-height: var(--line-height-normal);
  color: var(--color-text-primary);
}

.locked-notice code {
  background: rgba(0, 0, 0, 0.3);
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  font-family: var(--font-family-mono);
}

.source-settings {
  margin-top: var(--spacing-md);
  padding-top: var(--spacing-md);
  border-top: 1px solid rgba(255, 179, 0, 0.3);
}

.source-settings ul {
  margin: var(--spacing-sm) 0 0 0;
  padding-left: var(--spacing-xl);
}

.source-settings li {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}

.code-textarea {
  flex: 1;
  min-height: 400px;
  background: var(--color-bg-input);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: var(--spacing-lg);
  color: var(--color-text-primary);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-md);
  line-height: var(--line-height-normal);
  resize: vertical;
}

.code-textarea:focus {
  outline: none;
  border-color: var(--color-primary);
}

.code-textarea:disabled {
  background: var(--color-bg-panel);
  color: var(--color-text-secondary);
  cursor: not-allowed;
}

.editor-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  margin-top: var(--spacing-lg);
}

/* ============================================================================
   Empty State
   ============================================================================ */

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: var(--spacing-lg);
  opacity: 0.5;
}

.empty-text {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-medium);
  margin-bottom: var(--spacing-sm);
  color: var(--color-text-primary);
}

.empty-subtext {
  font-size: var(--font-size-md);
  color: var(--color-text-secondary);
  max-width: 300px;
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
  padding: var(--spacing-2xl);
  width: 100%;
  max-width: 420px;
  box-shadow: var(--shadow-lg);
}

.modal-title {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-semibold);
  margin-bottom: var(--spacing-xl);
  color: var(--color-text-primary);
}

.modal-subtitle {
  display: block;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-normal);
  color: var(--color-text-secondary);
  margin-top: var(--spacing-xs);
}

.form-group {
  margin-bottom: var(--spacing-lg);
}

.form-label {
  display: block;
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  margin-bottom: var(--spacing-sm);
  color: var(--color-text-primary);
}

.form-input,
.form-select {
  width: 100%;
  background: var(--color-bg-input);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  padding: var(--spacing-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
}

.form-input:focus,
.form-select:focus {
  outline: none;
  border-color: var(--color-primary);
}

.form-select {
  cursor: pointer;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--spacing-md);
  margin-top: var(--spacing-2xl);
}

/* ============================================================================
   Buttons
   ============================================================================ */

.btn-primary {
  padding: var(--spacing-md) var(--spacing-xl);
  background: var(--color-primary);
  border: none;
  border-radius: var(--radius-md);
  color: white;
  font-size: var(--font-size-md);
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
  padding: var(--spacing-md) var(--spacing-xl);
  background: transparent;
  border: 1px solid var(--color-border-medium);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-secondary:hover:not(:disabled) {
  background: var(--color-hover);
  border-color: var(--color-border-strong);
}

.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>