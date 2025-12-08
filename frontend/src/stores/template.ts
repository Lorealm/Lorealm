import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { templateApi } from '@/services/api/templates'
import { useAppStateStore } from './appState'
import type {
  ContextTemplate,
  SourceDefinition,
  TreeNode,
  Entry,
  Group,
  SaveState,
  NodePath,
  CreateNodeRequest,
  UpdateNodeRequest,
  NodeContent,
} from '@/types/context'
import { templateToTreeNodes, treeNodeToNodeContent, isGroup } from '@/types/context'

// Debounce delay for auto-save (ms)
const AUTOSAVE_DELAY = 1500

/**
 * Template store for managing the currently loaded template.
 *
 * Key design decisions:
 * - Nodes are managed individually via API (not bulk save)
 * - Tree structure is maintained locally for UI
 * - Path computation happens locally, synced with backend on operations
 * - Auto-save is per-node with debouncing
 */
export const useTemplateStore = defineStore('template', () => {
  const appState = useAppStateStore()

  // ==================== State ====================

  // Template metadata
  const templateId = ref<string | null>(null)
  const templateName = ref('')
  const templateDescription = ref('')
  const createdAt = ref<string | null>(null)
  const updatedAt = ref<string | null>(null)

  // Tree structure (for UI)
  const treeNodes = ref<TreeNode[]>([])

  // Sources
  const availableSources = ref<SourceDefinition[]>([])
  const usedSourceNodes = ref<[string, string][]>([]) // [source_id, node_id][]

  // Loading/saving state
  const isLoading = ref(false)
  const saveState = ref<SaveState>('saved')
  const lastSavedAt = ref<Date | null>(null)
  const saveError = ref<string | null>(null)

  // Track pending saves per node (for debouncing)
  const pendingSaves = ref<Map<string, number>>(new Map()) // nodeId -> timeoutId

  // Track node paths (computed from tree structure)
  const nodePaths = ref<Map<string, NodePath>>(new Map()) // nodeId -> path

  // ==================== Computed ====================

  const isLoaded = computed(() => templateId.value !== null)

  const hasUnsavedChanges = computed(
    () => saveState.value === 'unsaved' || pendingSaves.value.size > 0
  )

  // Get available source nodes (excluding already used ones)
  const availableSourceNodes = computed(() => {
    const used = new Set(usedSourceNodes.value.map(([s, n]) => `${s}:${n}`))

    return availableSources.value.flatMap((source) =>
      source.nodes
        .filter((node) => !used.has(`${source.source_id}:${node.node_id}`))
        .map((node) => ({
          source,
          node,
        }))
    )
  })

  // ==================== Path Management ====================

  /**
   * Recompute all node paths from the tree structure.
   * Called after tree modifications.
   */
  function recomputePaths(): void {
    const paths = new Map<string, NodePath>()

    function walkTree(nodes: TreeNode[], prefix: number[] = []): void {
      nodes.forEach((node, index) => {
        const path = [...prefix, index]
        paths.set(node.data.id, path)

        if (isGroup(node)) {
          walkTree(node.data.children, path)
        }
      })
    }

    walkTree(treeNodes.value)
    nodePaths.value = paths
  }

  /**
   * Get the path for a node by its ID
   */
  function getNodePath(nodeId: string): NodePath | null {
    return nodePaths.value.get(nodeId) || null
  }

  /**
   * Get the parent path for a node (path without the last index)
   */
  function getParentPath(nodeId: string): NodePath | null {
    const path = getNodePath(nodeId)
    if (!path || path.length === 0) return null
    if (path.length === 1) return null // Node is at root level, no parent path
    return path.slice(0, -1)
  }

  /**
   * Get the index of a node within its parent
   */
  function getNodeIndex(nodeId: string): number | null {
    const path = getNodePath(nodeId)
    if (!path || path.length === 0) return null
    return path[path.length - 1]
  }

  /**
   * Find the parent group ID for a node
   */
  function findParentGroupId(nodeId: string): string | null {
    const parentPath = getParentPath(nodeId)
    if (!parentPath) return null // Node is at root

    // Walk the tree to find the group at parentPath
    let current: TreeNode[] = treeNodes.value
    for (let i = 0; i < parentPath.length; i++) {
      const node = current[parentPath[i]]
      if (!node) return null
      if (i === parentPath.length - 1) {
        // This is the parent
        return isGroup(node) ? node.data.id : null
      }
      if (isGroup(node)) {
        current = node.data.children
      } else {
        return null
      }
    }
    return null
  }

  /**
   * Find a node by ID in the tree
   */
  function findNodeById(id: string, nodes: TreeNode[] = treeNodes.value): TreeNode | null {
    for (const node of nodes) {
      if (node.data.id === id) return node
      if (isGroup(node)) {
        const found = findNodeById(id, node.data.children)
        if (found) return found
      }
    }
    return null
  }

  /**
   * Find a group by ID
   */
  function findGroupById(id: string, nodes: TreeNode[] = treeNodes.value): Group | null {
    for (const node of nodes) {
      if (isGroup(node)) {
        if (node.data.id === id) return node.data
        const found = findGroupById(id, node.data.children)
        if (found) return found
      }
    }
    return null
  }

  /**
   * Find parent list containing a node
   */
  function findParentList(nodeId: string, nodes: TreeNode[] = treeNodes.value): TreeNode[] | null {
    for (const node of nodes) {
      if (node.data.id === nodeId) return nodes
      if (isGroup(node)) {
        const found = findParentList(nodeId, node.data.children)
        if (found) return found
      }
    }
    return null
  }

  // ==================== Template Operations ====================

  /**
   * Load a template by ID
   */
  async function loadTemplate(id: string): Promise<boolean> {
    if (hasUnsavedChanges.value) {
      // Could prompt user here, but for now just warn
      console.warn('Loading new template while there are unsaved changes')
    }

    isLoading.value = true
    saveError.value = null

    try {
      const response = await templateApi.getTemplate(id)

      // Set template data
      templateId.value = response.template.id
      templateName.value = response.template.name
      templateDescription.value = response.template.description
      createdAt.value = response.template.created_at
      updatedAt.value = response.template.updated_at

      // Convert nodes to tree structure
      treeNodes.value = templateToTreeNodes(response.template)

      // Set sources
      console.log('Loaded available_sources:', response.available_sources)
      console.log('Loaded used_source_nodes:', response.used_source_nodes)
      availableSources.value = response.available_sources
      usedSourceNodes.value = response.used_source_nodes

      // Recompute paths
      recomputePaths()

      // Update app state
      appState.setActiveTemplate(id)

      // Reset save state
      saveState.value = 'saved'
      lastSavedAt.value = new Date()
      pendingSaves.value.clear()

      return true
    } catch (err) {
      console.error('Failed to load template:', err)
      saveError.value = err instanceof Error ? err.message : 'Failed to load template'
      return false
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Reload the current template
   */
  async function reloadTemplate(): Promise<boolean> {
    if (!templateId.value) return false
    return loadTemplate(templateId.value)
  }

  /**
   * Update template metadata (name, description)
   */
  async function updateTemplateMetadata(name?: string, description?: string): Promise<boolean> {
    if (!templateId.value) return false

    saveState.value = 'saving'

    try {
      const response = await templateApi.updateTemplate(templateId.value, {
        name,
        description,
      })

      if (name !== undefined) templateName.value = response.name
      if (description !== undefined) templateDescription.value = response.description
      updatedAt.value = response.updated_at

      saveState.value = 'saved'
      lastSavedAt.value = new Date()

      return true
    } catch (err) {
      console.error('Failed to update template:', err)
      saveState.value = 'error'
      saveError.value = err instanceof Error ? err.message : 'Failed to update template'
      return false
    }
  }

  /**
   * Unload the current template
   */
  function unloadTemplate(): void {
    // Cancel any pending saves
    pendingSaves.value.forEach((timeoutId) => clearTimeout(timeoutId))
    pendingSaves.value.clear()

    // Reset state
    templateId.value = null
    templateName.value = ''
    templateDescription.value = ''
    createdAt.value = null
    updatedAt.value = null
    treeNodes.value = []
    availableSources.value = []
    usedSourceNodes.value = []
    nodePaths.value.clear()
    saveState.value = 'saved'
    saveError.value = null
  }

  // ==================== Node Operations ====================

  /**
   * Create a new node
   */
  async function createNode(
    title: string,
    content: NodeContent,
    parentPath?: NodePath,
    index?: number
  ): Promise<string | null> {
    if (!templateId.value) return null

    saveState.value = 'saving'

    try {
      const request: CreateNodeRequest = {
        title,
        content,
        parent_path: parentPath,
        index,
      }

      const response = await templateApi.createNode(templateId.value, request)

      // Reload to get fresh tree (simplest approach)
      // In future, could update tree locally
      await reloadTemplate()

      saveState.value = 'saved'
      lastSavedAt.value = new Date()

      return response.node.id
    } catch (err) {
      console.error('Failed to create node:', err)
      saveState.value = 'error'
      saveError.value = err instanceof Error ? err.message : 'Failed to create node'
      return null
    }
  }

  /**
   * Update a node (with debouncing for content changes)
   */
  function scheduleNodeUpdate(nodeId: string, updates: UpdateNodeRequest): void {
    if (!templateId.value) return

    // Cancel any pending save for this node
    const existingTimeout = pendingSaves.value.get(nodeId)
    if (existingTimeout) {
      clearTimeout(existingTimeout)
    }

    saveState.value = 'unsaved'

    // Schedule new save
    const timeoutId = window.setTimeout(async () => {
      pendingSaves.value.delete(nodeId)
      await updateNodeNow(nodeId, updates)
    }, AUTOSAVE_DELAY)

    pendingSaves.value.set(nodeId, timeoutId)
  }

  /**
   * Update a node immediately
   */
  async function updateNodeNow(nodeId: string, updates: UpdateNodeRequest): Promise<boolean> {
    if (!templateId.value) return false

    saveState.value = 'saving'

    try {
      await templateApi.updateNode(templateId.value, nodeId, updates)

      saveState.value = pendingSaves.value.size > 0 ? 'unsaved' : 'saved'
      lastSavedAt.value = new Date()

      return true
    } catch (err) {
      console.error('Failed to update node:', err)
      saveState.value = 'error'
      saveError.value = err instanceof Error ? err.message : 'Failed to update node'
      return false
    }
  }

  /**
   * Delete a node
   */
  async function deleteNode(nodeId: string): Promise<boolean> {
    if (!templateId.value) return false

    // Cancel any pending save for this node
    const existingTimeout = pendingSaves.value.get(nodeId)
    if (existingTimeout) {
      clearTimeout(existingTimeout)
      pendingSaves.value.delete(nodeId)
    }

    saveState.value = 'saving'

    try {
      await templateApi.deleteNode(templateId.value, nodeId)

      // Reload to get fresh tree
      await reloadTemplate()

      saveState.value = 'saved'
      lastSavedAt.value = new Date()

      return true
    } catch (err) {
      console.error('Failed to delete node:', err)
      saveState.value = 'error'
      saveError.value = err instanceof Error ? err.message : 'Failed to delete node'
      return false
    }
  }

  /**
   * Move a node
   */
  async function moveNode(
    nodeId: string,
    newParentPath: NodePath | undefined,
    newIndex: number
  ): Promise<boolean> {
    if (!templateId.value) return false

    saveState.value = 'saving'

    try {
      await templateApi.moveNode(templateId.value, nodeId, {
        new_parent_path: newParentPath,
        new_index: newIndex,
      })

      // Reload to get fresh tree
      await reloadTemplate()

      saveState.value = 'saved'
      lastSavedAt.value = new Date()

      return true
    } catch (err) {
      console.error('Failed to move node:', err)
      saveState.value = 'error'
      saveError.value = err instanceof Error ? err.message : 'Failed to move node'
      return false
    }
  }

  /**
   * Toggle node enabled state
   */
  async function toggleNode(nodeId: string): Promise<boolean> {
    if (!templateId.value) return false

    try {
      const response = await templateApi.toggleNode(templateId.value, nodeId)

      // Update local state
      const node = findNodeById(nodeId)
      if (node) {
        if (isGroup(node)) {
          node.data.enabled = response.enabled
        } else {
          node.data.enabled = response.enabled
        }
      }

      return true
    } catch (err) {
      console.error('Failed to toggle node:', err)
      saveError.value = err instanceof Error ? err.message : 'Failed to toggle node'
      return false
    }
  }

  // ==================== Source Node Helpers ====================

  /**
   * Add a source node to the template
   */
  async function addSourceNode(
    sourceId: string,
    nodeId: string,
    title: string,
    settings: Record<string, unknown>,
    parentPath?: NodePath,
    index?: number
  ): Promise<string | null> {
    // Debug logging
    console.log('addSourceNode called with:', {
      sourceId,
      nodeId,
      title,
      settings,
      parentPath,
      index,
    })

    if (!sourceId || !nodeId) {
      console.error('addSourceNode: Missing sourceId or nodeId!', { sourceId, nodeId })
      saveError.value = 'Missing source or node ID'
      return null
    }

    const content: NodeContent = {
      type: 'source',
      source_id: sourceId,
      node_id: nodeId,
      settings,
    }

    console.log('Creating node with content:', content)

    return createNode(title, content, parentPath, index)
  }

  /**
   * Add a text entry to the template
   */
  async function addTextEntry(
    title: string,
    role: 'system' | 'user' | 'assistant',
    content: string,
    parentPath?: NodePath,
    index?: number
  ): Promise<string | null> {
    const nodeContent: NodeContent = {
      type: 'text',
      role,
      content,
    }

    return createNode(title, nodeContent, parentPath, index)
  }

  /**
   * Add a group to the template
   */
  async function addGroup(
    title: string,
    description?: string,
    parentPath?: NodePath,
    index?: number
  ): Promise<string | null> {
    const content: NodeContent = {
      type: 'group',
      children: [],
      always_include_wrappers: false,
    }

    return createNode(title, content, parentPath, index)
  }

  // ==================== Entry Content Helpers ====================

  /**
   * Update entry content (debounced)
   */
  function updateEntryContent(entry: Entry, newContent: string): void {
    // Update local state immediately
    entry.content = newContent

    // Schedule save
    scheduleNodeUpdate(entry.id, {
      content: treeNodeToNodeContent({ type: 'entry', data: entry }),
    })
  }

  /**
   * Update entry title
   */
  function updateEntryTitle(entry: Entry, newTitle: string): void {
    entry.name = newTitle
    scheduleNodeUpdate(entry.id, { title: newTitle })
  }

  /**
   * Update group title/description
   */
  function updateGroup(group: Group, title?: string, description?: string): void {
    if (title !== undefined) group.name = title
    if (description !== undefined) group.description = description

    scheduleNodeUpdate(group.id, {
      title: title ?? group.name,
      description: description ?? group.description,
    })
  }

  // ==================== Flush Pending Saves ====================

  /**
   * Force save all pending changes immediately
   */
  async function flushPendingSaves(): Promise<void> {
    const pending = Array.from(pendingSaves.value.entries())

    // Cancel all timeouts
    pending.forEach(([_, timeoutId]) => clearTimeout(timeoutId))
    pendingSaves.value.clear()

    // Note: We would need to track the actual updates to replay them
    // For now, just reload to ensure consistency
    if (pending.length > 0 && templateId.value) {
      await reloadTemplate()
    }
  }

  // ==================== Cleanup on unmount ====================

  // Watch for store disposal to cleanup timeouts
  // Note: This won't trigger in Pinia, but good practice

  return {
    // State
    templateId,
    templateName,
    templateDescription,
    createdAt,
    updatedAt,
    treeNodes,
    availableSources,
    usedSourceNodes,
    isLoading,
    saveState,
    lastSavedAt,
    saveError,

    // Computed
    isLoaded,
    hasUnsavedChanges,
    availableSourceNodes,

    // Path management
    getNodePath,
    findNodeById,
    findGroupById,
    findParentList,
    recomputePaths,

    // Template operations
    loadTemplate,
    reloadTemplate,
    updateTemplateMetadata,
    unloadTemplate,

    // Node operations
    createNode,
    scheduleNodeUpdate,
    updateNodeNow,
    deleteNode,
    moveNode,
    toggleNode,

    // Helpers
    addSourceNode,
    addTextEntry,
    addGroup,
    updateEntryContent,
    updateEntryTitle,
    updateGroup,
    flushPendingSaves,
  }
})
