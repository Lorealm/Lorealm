<script setup lang="ts">
import { inject, ref } from 'vue'
import draggable from 'vuedraggable'
import type { TreeNode, Group, Entry, ContextEditorContext } from '@/types/context'

// ============================================================================
// Props & Emits
// ============================================================================

const props = defineProps<{
  nodes: TreeNode[]
  parentGroupId: string | null
  depth: number
}>()

const emit = defineEmits<{
  (e: 'update:nodes', nodes: TreeNode[]): void
}>()

// Inject the shared context from parent
const ctx = inject<ContextEditorContext>('contextEditor')!

// Track the node being dragged
const draggedNodeId = ref<string | null>(null)

// ============================================================================
// Helpers
// ============================================================================

function isGroup(node: TreeNode): node is { type: 'group'; data: Group } {
  return node.type === 'group'
}

function isSourceNode(entry: Entry): boolean {
  return !!entry.sourceId && !!entry.nodeId
}

function isCollapsed(groupId: string): boolean {
  return ctx.collapsedGroups.has(groupId)
}

function isSelected(entryId: string): boolean {
  return ctx.selectedEntry?.id === entryId
}

// Handle drag start
function onDragStart(evt: any): void {
  const nodeId = evt.item?.dataset?.nodeId
  if (nodeId) {
    draggedNodeId.value = nodeId
    ctx.onDragStart(nodeId)
  }
}

// Handle drag end - called when drag completes
function onDragEnd(evt: any): void {
  const nodeId = evt.item?.dataset?.nodeId
  if (!nodeId) {
    draggedNodeId.value = null
    return
  }
  
  // Get the new index from the event
  const newIndex = evt.newIndex
  
  // Determine the target parent group ID
  // The 'to' element has our data-group-id attribute
  const toGroupId = evt.to?.dataset?.groupId || null
  const targetParentGroupId = toGroupId === '' ? null : toGroupId
  
  console.log('Drag end:', { nodeId, newIndex, targetParentGroupId, evt })
  
  // Call the context handler to persist the move
  ctx.onNodeMoved(nodeId, targetParentGroupId, newIndex)
  
  draggedNodeId.value = null
}

// Check if a move is allowed (for depth constraints)
function checkMove(evt: any): boolean {
  const draggedEl = evt.dragged
  const toEl = evt.to
  
  const draggedId = draggedEl?.dataset?.nodeId
  const draggedType = draggedEl?.dataset?.nodeType
  
  // Entries can always be moved
  if (draggedType === 'entry') return true
  
  // For groups, check depth constraints
  if (draggedType === 'group' && draggedId) {
    const draggedGroup = ctx.findGroupById(draggedId)
    if (!draggedGroup) return true
    
    const targetGroupId = toEl?.dataset?.groupId
    let targetDepth = -1
    
    if (targetGroupId) {
      const targetGroup = ctx.findGroupById(targetGroupId)
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
    
    return wouldBeDepth <= ctx.maxNestingDepth
  }
  
  return true
}

// Update children of a group
function updateChildren(group: Group, newChildren: TreeNode[]) {
  group.children = newChildren
}
</script>

<template>
  <draggable
    :model-value="nodes"
    @update:model-value="emit('update:nodes', $event)"
    item-key="data.id"
    group="tree-nodes"
    handle=".drag-handle"
    :animation="150"
    ghost-class="drag-ghost"
    chosen-class="drag-chosen"
    drag-class="drag-dragging"
    :move="checkMove"
    @start="onDragStart"
    @end="onDragEnd"
    :data-group-id="parentGroupId || ''"
    :data-depth="depth"
    class="draggable-list"
  >
    <template #item="{ element: node }">
      <div 
        class="tree-item" 
        :data-node-id="node.data.id" 
        :data-node-type="node.type"
      >
        <!-- ============ GROUP ============ -->
        <template v-if="isGroup(node)">
          <div class="tree-group">
            <div class="group-header">
              <span class="drag-handle">⋮⋮</span>
              <span 
                class="group-toggle" 
                :class="{ collapsed: isCollapsed(node.data.id) }"
                @click="ctx.toggleGroup(node.data.id)"
              >▼</span>
              <span class="group-name">{{ node.data.name }}</span>
              <span v-if="node.data.depth > 0" class="depth-badge">
                L{{ node.data.depth }}
              </span>
              <div class="group-actions">
                <button
                  v-if="node.data.depth < ctx.maxNestingDepth - 1"
                  class="btn-icon"
                  @click.stop="ctx.openAddGroupModal(node.data.id)"
                  title="Add subgroup"
                >📁+</button>
                <button
                  class="btn-icon"
                  @click.stop="ctx.openAddEntryModal(node.data.id)"
                  title="Add entry"
                >📝+</button>
                <button class="btn-icon" @click.stop="ctx.openEditGroupModal(node.data)">✏️</button>
                <button class="btn-icon" @click.stop="ctx.deleteGroup(node.data)">🗑️</button>
              </div>
            </div>

            <!-- Recursively render children -->
            <div v-show="!isCollapsed(node.data.id)" class="group-contents">
              <TreeNodeList
                :nodes="node.data.children"
                :parent-group-id="node.data.id"
                :depth="depth + 1"
                @update:nodes="updateChildren(node.data, $event)"
              />
              <div v-if="node.data.children.length === 0" class="empty-group">
                Drop items here
              </div>
            </div>
          </div>
        </template>

        <!-- ============ ENTRY ============ -->
        <div
          v-else
          class="tree-entry"
          :class="{
            active: isSelected(node.data.id),
            locked: (node.data as Entry).locked,
            'source-node': isSourceNode(node.data as Entry),
          }"
          @click="ctx.selectEntry(node.data as Entry)"
        >
          <span class="drag-handle" :class="{ disabled: (node.data as Entry).locked }">
            <template v-if="isSourceNode(node.data as Entry)">📡</template>
            <template v-else-if="(node.data as Entry).locked">🔒</template>
            <template v-else>⋮⋮</template>
          </span>
          <span :class="'entry-role role-' + (node.data as Entry).role.toLowerCase()">
            {{ (node.data as Entry).role }}
          </span>
          <span class="entry-name">
            {{ (node.data as Entry).name }}
            <span v-if="isSourceNode(node.data as Entry)" class="source-badge">
              {{ (node.data as Entry).sourceId }}
            </span>
            <span v-else-if="(node.data as Entry).computedSource" class="computed-badge">Auto</span>
          </span>
          <div class="entry-actions">
            <div
              v-if="!(node.data as Entry).locked"
              class="entry-toggle"
              :class="{ enabled: (node.data as Entry).enabled }"
              @click.stop="ctx.toggleEntry(node.data as Entry)"
            ></div>
            <button class="btn-icon-small" @click.stop="ctx.deleteEntry(node.data as Entry)">×</button>
          </div>
        </div>
      </div>
    </template>
  </draggable>
</template>

<script lang="ts">
// This allows the component to reference itself recursively
export default {
  name: 'TreeNodeList',
}
</script>

<style scoped>
/* ============================================================================
   Draggable List
   ============================================================================ */

.draggable-list {
  min-height: 20px;
}

.tree-item {
  margin-bottom: var(--spacing-xs);
}

/* ============================================================================
   Groups
   ============================================================================ */

.tree-group {
  background: var(--color-bg-elevated-2);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.group-header {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-md) var(--spacing-md);
  background: var(--color-bg-overlay);
  border-bottom: 1px solid var(--color-border-subtle);
}

.group-toggle {
  cursor: pointer;
  user-select: none;
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
  transition: transform var(--transition-fast);
  width: 16px;
  text-align: center;
}

.group-toggle.collapsed {
  transform: rotate(-90deg);
}

.group-name {
  font-weight: var(--font-weight-medium);
  font-size: var(--font-size-md);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-primary);
}

.group-actions {
  display: flex;
  gap: var(--spacing-xs);
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.group-header:hover .group-actions {
  opacity: 1;
}

.group-contents {
  padding: var(--spacing-sm);
}

.empty-group {
  padding: var(--spacing-lg);
  text-align: center;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  border: 2px dashed var(--color-border-medium);
  border-radius: var(--radius-md);
  margin: var(--spacing-xs);
}

.depth-badge {
  font-size: var(--font-size-xs);
  background: var(--color-hover);
  color: var(--color-text-secondary);
  padding: 2px 6px;
  border-radius: var(--radius-full);
  font-weight: var(--font-weight-medium);
}

/* ============================================================================
   Entries
   ============================================================================ */

.tree-entry {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-sm) var(--spacing-md);
  background: var(--color-bg-elevated-2);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.tree-entry:hover {
  background: var(--color-hover);
  border-color: var(--color-border-medium);
}

.tree-entry.active {
  background: var(--color-active);
  border-color: var(--color-primary);
}

.tree-entry.locked {
  background: rgba(255, 179, 0, 0.05);
  border-left: 3px solid var(--color-warning);
}

.tree-entry.source-node {
  background: rgba(139, 92, 246, 0.05);
  border-left: 3px solid #a78bfa;
}

.tree-entry.source-node.active {
  background: rgba(139, 92, 246, 0.15);
  border-color: #a78bfa;
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

.entry-name {
  flex: 1;
  font-size: var(--font-size-base);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--color-text-primary);
}

.computed-badge {
  font-size: var(--font-size-xs);
  background: var(--color-info-bg);
  color: var(--color-info-text);
  padding: 1px 5px;
  border-radius: var(--radius-sm);
  margin-left: var(--spacing-sm);
  font-weight: var(--font-weight-medium);
}

.source-badge {
  font-size: var(--font-size-xs);
  background: rgba(139, 92, 246, 0.2);
  color: #a78bfa;
  padding: 1px 5px;
  border-radius: var(--radius-sm);
  margin-left: var(--spacing-sm);
  font-weight: var(--font-weight-medium);
}

.entry-actions {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.tree-entry:hover .entry-actions {
  opacity: 1;
}

.entry-toggle {
  width: 28px;
  height: 16px;
  background: var(--color-bg-panel-solid);
  border-radius: var(--radius-lg);
  position: relative;
  cursor: pointer;
  transition: background var(--transition-fast);
  border: 1px solid var(--color-border-medium);
}

.entry-toggle::after {
  content: '';
  position: absolute;
  width: 12px;
  height: 12px;
  background: var(--color-text-secondary);
  border-radius: 50%;
  top: 1px;
  left: 1px;
  transition: transform var(--transition-fast), background var(--transition-fast);
}

.entry-toggle.enabled {
  background: var(--color-success);
  border-color: var(--color-success);
}

.entry-toggle.enabled::after {
  transform: translateX(12px);
  background: white;
}

/* ============================================================================
   Drag Handle & States
   ============================================================================ */

.drag-handle {
  cursor: grab;
  color: var(--color-text-muted);
  user-select: none;
  font-size: var(--font-size-sm);
  letter-spacing: -2px;
  padding: var(--spacing-xs);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.drag-handle:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.drag-handle.disabled {
  cursor: default;
  opacity: 0.5;
}

.drag-handle:active {
  cursor: grabbing;
}

/* Drag states - applied by SortableJS */
:deep(.drag-ghost) {
  opacity: 0.4;
  background: var(--color-primary);
}

:deep(.drag-chosen) {
  opacity: 0.9;
}

:deep(.drag-dragging) {
  opacity: 1;
  box-shadow: var(--shadow-lg);
}

/* ============================================================================
   Icon Buttons
   ============================================================================ */

.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-md);
  padding: var(--spacing-xs) var(--spacing-sm);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.btn-icon:hover {
  background: var(--color-hover);
}

.btn-icon-small {
  background: none;
  border: none;
  cursor: pointer;
  font-size: var(--font-size-lg);
  color: var(--color-text-secondary);
  padding: 2px var(--spacing-xs);
  line-height: 1;
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.btn-icon-small:hover {
  color: var(--color-error);
  background: var(--color-error-bg);
}
</style>