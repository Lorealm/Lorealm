<script setup lang="ts">
import api from '@/services/api'
import { ref, computed, reactive } from 'vue'

interface Group {
  name: string
  description?: string
  color?: string
  id: string
  order: number
  enabled: boolean
  entries: Entry[]
}

interface Entry {
  name: string
  role: 'User' | 'Assistant' | 'System'
  id: string
  order: number
  enabled: boolean
  content: string
}

interface DragState {
  draggedItem: (Entry | Group) | null
  draggedType: 'entry' | 'group' | null
  dragOverItem: string | null
  dragOverPosition: 'before' | 'after' | 'inside' | null
}

interface RootItem {
  type: 'entry' | 'group'
  item: Entry | Group
  order: number
}

interface Template {
  name: string
  description: string
}

interface NewGroup {
  name: string
  description: string
  color: string
}

interface NewEntry {
  name: string
  role: 'User' | 'Assistant' | 'System'
  groupId: string
}

type DragOverMode = 'reorder' | 'into' | 'root' | null
type DragPosition = 'before' | 'after' | 'inside' | null
type TargetType = 'entry' | 'group' | 'root'

// Reactive state
const currentView = ref<string>('edit')
const currentTemplate = reactive<Template>({
  name: 'My Template',
  description: 'A custom template for AI interactions',
})

// Mark your function as async
async function fetchContextTemplates() {
  try {
    const response = await api.get("/state/context_templates/");
    console.log(response.data); // Most HTTP libraries put the result in `.data`
  } catch (error) {
    console.error("Failed to fetch context templates:", error);
  }
}

fetchContextTemplates();

const groups = ref<Group[]>([
  {
    id: 'group-1',
    name: 'System Context',
    description: 'Core system instructions',
    order: 1,
    enabled: true,
    entries: [
      {
        id: 'entry-3',
        name: 'Base Instructions',
        role: 'System',
        order: 0,
        enabled: true,
        content: 'You are a helpful AI assistant...',
      },
      {
        id: 'entry-4',
        name: 'Format Guidelines',
        role: 'System',
        order: 1,
        enabled: true,
        content: 'Please format your responses...',
      },
    ],
  },
  {
    id: 'group-2',
    name: 'Character Info',
    description: 'Character definitions and traits',
    order: 3,
    enabled: true,
    entries: [
      {
        id: 'entry-5',
        name: 'Main Character',
        role: 'User',
        order: 0,
        enabled: true,
        content: 'Character name: ...',
      },
    ],
  },
])

const ungroupedEntries = ref<Entry[]>([
  {
    id: 'entry-1',
    name: 'Opening Message',
    role: 'Assistant',
    order: 0,
    enabled: true,
    content: 'Hello! How can I help you today?',
  },
  {
    id: 'entry-2',
    name: 'User Context',
    role: 'User',
    order: 2,
    enabled: true,
    content: 'I need help with...',
  },
])

const selectedEntry = ref<Entry | null>(null)
const contentChanged = ref<boolean>(false)

// Drag state
const draggedItem = ref<(Entry | Group) | null>(null)
const draggedType = ref<'entry' | 'group' | null>(null)
const draggedFromGroup = ref<boolean>(false)
const dragOverItem = ref<(Entry | Group) | null>(null)
const dragOverType = ref<TargetType | null>(null)
const dragOverMode = ref<DragOverMode>(null)
const dragPosition = ref<DragPosition>(null)

// Modals
const showAddGroupModal = ref<boolean>(false)
const showAddEntryModal = ref<boolean>(false)
const editingGroupId = ref<string | null>(null)

// Form data
const newGroup = reactive<NewGroup>({
  name: '',
  description: '',
  color: '',
})

const newEntry = reactive<NewEntry>({
  name: '',
  role: 'System',
  groupId: '',
})

const collapsedGroups = ref<Record<string, boolean>>({})

// Computed
const rootItems = computed<RootItem[]>(() => {
  const items: RootItem[] = []

  // Add ungrouped entries
  ungroupedEntries.value.forEach((entry: Entry) => {
    items.push({
      type: 'entry',
      item: entry,
      order: entry.order,
    })
  })

  // Add groups
  groups.value.forEach((group: Group) => {
    items.push({
      type: 'group',
      item: group,
      order: group.order,
    })
  })

  return items.sort((a: RootItem, b: RootItem) => a.order - b.order)
})

// Drag and Drop Methods
const startDrag = (
  item: Entry | Group,
  type: 'entry' | 'group',
  fromGroup: boolean = false
): void => {
  draggedItem.value = item
  draggedType.value = type
  draggedFromGroup.value = fromGroup

  // Add visual feedback
  document.body.classList.add(`dragging-${type}`)
}

const endDrag = (): void => {
  draggedItem.value = null
  draggedType.value = null
  draggedFromGroup.value = false
  clearDragOver()

  // Remove visual feedback
  document.body.classList.remove('dragging-entry', 'dragging-group')
}

const onDragOver = (
  targetItem: Entry | Group | null,
  targetType: TargetType,
  event: DragEvent
): void => {
  event.preventDefault()

  if (!draggedItem.value) return

  // More forgiving position detection
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const y = event.clientY - rect.top
  const height = rect.height

  let position: DragPosition = 'after'
  let mode: DragOverMode = 'reorder'

  if (targetType === 'group' && draggedType.value === 'entry') {
    // Much larger zones for groups - 60% inside zone
    if (y < height * 0.2) {
      position = 'before'
      mode = 'reorder'
    } else if (y > height * 0.8) {
      position = 'after'
      mode = 'reorder'
    } else {
      position = 'inside'
      mode = 'into'
    }
  } else if (targetType === 'entry') {
    // More forgiving zones for entries - 40% each side
    position = y < height * 0.6 ? 'before' : 'after'
    mode = 'reorder'
  } else if (targetType === 'group' && draggedType.value === 'group') {
    // Groups can only be reordered
    position = y < height * 0.5 ? 'before' : 'after'
    mode = 'reorder'
  } else if (targetType === 'root') {
    mode = 'root'
  }

  // More permissive validation
  if (draggedType.value === 'group' && mode === 'into') return

  dragOverItem.value = targetItem
  dragOverType.value = targetType
  dragOverMode.value = mode
  dragPosition.value = position
}

const clearDragOver = (): void => {
  dragOverItem.value = null
  dragOverType.value = null
  dragOverMode.value = null
  dragPosition.value = null
}

const handleDrop = (
  targetItem: Entry | Group | null,
  targetType: TargetType,
  inGroup: boolean = false
): void => {
  if (!draggedItem.value || !dragOverMode.value) return

  const draggedItemValue = draggedItem.value
  const draggedTypeValue = draggedType.value
  const position = dragPosition.value

  if (draggedTypeValue === 'group' && dragOverMode.value === 'reorder' && position) {
    reorderAtRootLevel(draggedItemValue as Group, targetItem!, position)
  } else if (draggedTypeValue === 'entry') {
    if (dragOverMode.value === 'into' && targetItem && 'entries' in targetItem) {
      moveEntryToGroup(draggedItemValue as Entry, targetItem as Group)
    } else if (dragOverMode.value === 'root') {
      moveEntryToRoot(draggedItemValue as Entry)
    } else if (dragOverMode.value === 'reorder' && position) {
      if (inGroup) {
        reorderEntriesInGroup(draggedItemValue as Entry, targetItem as Entry, position)
      } else {
        reorderAtRootLevel(draggedItemValue as Entry, targetItem!, position)
      }
    }
  }

  endDrag()
}

// New method for reordering at root level (both groups and entries)
const reorderAtRootLevel = (
  draggedItem: Entry | Group,
  targetItem: Entry | Group,
  position: 'before' | 'after' | 'inside'
): void => {
  // Remove from current location
  removeFromCurrentLocation(draggedItem)

  // Find target order
  let targetOrder: number
  if ('entries' in targetItem) {
    // Target is a group
    targetOrder = (targetItem as Group).order
  } else {
    // Target is an ungrouped entry
    targetOrder = (targetItem as Entry).order
  }

  // Adjust for position
  if (position === 'after') targetOrder++

  // Update orders of existing items
  updateRootOrdersForInsertion(targetOrder)

  // Add item at new location
  if ('entries' in draggedItem) {
    // Moving a group
    ;(draggedItem as Group).order = targetOrder
  } else {
    // Moving an entry to root level
    ;(draggedItem as Entry).order = targetOrder
    ungroupedEntries.value.push(draggedItem as Entry)
  }
}

const updateRootOrdersForInsertion = (insertOrder: number): void => {
  // Update ungrouped entries
  ungroupedEntries.value.forEach((entry: Entry) => {
    if (entry.order >= insertOrder) entry.order++
  })

  // Update groups
  groups.value.forEach((group: Group) => {
    if (group.order >= insertOrder) group.order++
  })
}

const removeFromCurrentLocation = (item: Entry | Group): void => {
  if ('entries' in item) {
    // It's a group - already in the groups array, no need to remove
    return
  }

  // It's an entry - remove from current location
  const entry = item as Entry

  // Remove from ungrouped entries
  const ungroupedIndex = ungroupedEntries.value.findIndex((e: Entry) => e.id === entry.id)
  if (ungroupedIndex !== -1) {
    ungroupedEntries.value.splice(ungroupedIndex, 1)
    return
  }

  // Remove from groups
  for (const group of groups.value) {
    const entryIndex = group.entries.findIndex((e: Entry) => e.id === entry.id)
    if (entryIndex !== -1) {
      group.entries.splice(entryIndex, 1)
      // Reorder remaining entries in group
      group.entries.forEach((e: Entry, index: number) => {
        e.order = index
      })
      break
    }
  }
}

const moveEntryToGroup = (entry: Entry, targetGroup: Group): void => {
  removeFromCurrentLocation(entry)

  // Add to target group at the end
  const newOrder = targetGroup.entries.length
  const newEntry: Entry = { ...entry, order: newOrder }
  targetGroup.entries.push(newEntry)
}

const moveEntryToRoot = (entry: Entry): void => {
  removeFromCurrentLocation(entry)

  // Add to root level at the end
  const maxOrder = Math.max(
    ...ungroupedEntries.value.map((e: Entry) => e.order),
    ...groups.value.map((g: Group) => g.order),
    -1
  )
  const newEntry: Entry = { ...entry, order: maxOrder + 1 }
  ungroupedEntries.value.push(newEntry)
}

const reorderEntriesInGroup = (
  draggedEntry: Entry,
  targetEntry: Entry,
  position: 'before' | 'after' | 'inside'
): void => {
  // Find which group contains the target entry
  const group = groups.value.find((g: Group) =>
    g.entries.some((e: Entry) => e.id === targetEntry.id)
  )
  if (!group) return

  // Remove dragged entry from current location
  removeFromCurrentLocation(draggedEntry)

  // Find target position in group
  const targetIndex = group.entries.findIndex((e: Entry) => e.id === targetEntry.id)
  const insertIndex = position === 'before' ? targetIndex : targetIndex + 1

  // Insert entry at specific position
  const newEntry: Entry = { ...draggedEntry, order: insertIndex }
  group.entries.splice(insertIndex, 0, newEntry)

  // Reorder all entries in the group
  group.entries.forEach((entry: Entry, index: number) => {
    entry.order = index
  })
}

// UI Methods
const selectEntry = (entry: Entry): void => {
  selectedEntry.value = entry
  contentChanged.value = false
}

const toggleGroup = (groupId: string): void => {
  collapsedGroups.value[groupId] = !collapsedGroups.value[groupId]
}

const toggleEntry = (entry: Entry): void => {
  entry.enabled = !entry.enabled
}

// CRUD Methods
const addGroup = (): void => {
  const maxOrder = Math.max(
    ...ungroupedEntries.value.map((e: Entry) => e.order),
    ...groups.value.map((g: Group) => g.order),
    -1
  )

  const group: Group = {
    id: `group-${Date.now()}`,
    name: newGroup.name || 'New Group',
    description: newGroup.description,
    color: newGroup.color,
    order: maxOrder + 1,
    enabled: true,
    entries: [],
  }

  groups.value.push(group)
  showAddGroupModal.value = false
  resetGroupForm()
}

const addEntry = (): void => {
  const entry: Entry = {
    id: `entry-${Date.now()}`,
    name: newEntry.name || 'New Entry',
    role: newEntry.role,
    order: 0,
    enabled: true,
    content: '',
  }

  if (newEntry.groupId) {
    const group = groups.value.find((g: Group) => g.id === newEntry.groupId)
    if (group) {
      entry.order = group.entries.length
      group.entries.push(entry)
    }
  } else {
    const maxOrder = Math.max(
      ...ungroupedEntries.value.map((e: Entry) => e.order),
      ...groups.value.map((g: Group) => g.order),
      -1
    )
    entry.order = maxOrder + 1
    ungroupedEntries.value.push(entry)
  }

  showAddEntryModal.value = false
  resetEntryForm()
}

const editGroup = (group: Group): void => {
  editingGroupId.value = group.id
  newGroup.name = group.name
  newGroup.description = group.description || ''
  newGroup.color = group.color || ''
  showAddGroupModal.value = true
}

const deleteGroup = (group: Group): void => {
  // Move entries to root level
  group.entries.forEach((entry: Entry) => {
    const maxOrder = Math.max(
      ...ungroupedEntries.value.map((e: Entry) => e.order),
      ...groups.value.map((g: Group) => g.order),
      -1
    )
    entry.order = maxOrder + 1
    ungroupedEntries.value.push(entry)
  })

  // Remove group
  const index = groups.value.findIndex((g: Group) => g.id === group.id)
  if (index !== -1) {
    groups.value.splice(index, 1)
  }
}

const saveContent = (): void => {
  console.log('Saving content for entry:', selectedEntry.value?.id)
  contentChanged.value = false
}

const revertChanges = (): void => {
  contentChanged.value = false
}

const resetGroupForm = (): void => {
  newGroup.name = ''
  newGroup.description = ''
  newGroup.color = ''
  editingGroupId.value = null
}

const resetEntryForm = (): void => {
  newEntry.name = ''
  newEntry.role = 'System'
  newEntry.groupId = ''
}
</script>

<template>
  <!-- Template Editor -->
  <div class="template-editor-container">
    <!-- Hierarchy Panel -->
    <div class="template-hierarchy">
      <div class="hierarchy-header">
        <div class="template-title">{{ currentTemplate.name }}</div>
        <div class="template-description">{{ currentTemplate.description }}</div>
      </div>

      <div class="hierarchy-actions">
        <button class="btn-small" @click="showAddGroupModal = true">+ Add Group</button>
        <button class="btn-small" @click="showAddEntryModal = true">+ Add Entry</button>
      </div>

      <div class="hierarchy-tree">
        <!-- Root drop zone for moving entries out of groups -->
        <!-- <div
            v-if="draggedItem && draggedType === 'entry' && draggedFromGroup"
            class="root-drop-zone"
            :class="{ 'drag-over': dragOverType === 'root' && dragOverMode === 'root' }"
            @dragover.prevent="onDragOver(null, 'root', $event)"
            @dragenter.prevent
            @dragleave="clearDragOver"
            @drop="handleDrop(null, 'root')"
          >
            <span>Drop here to move to root level</span>
          </div> -->

        <!-- Combined root items (groups and entries intermixed) -->
        <div
          v-for="rootItem in rootItems"
          :key="`${rootItem.type}-${rootItem.item.id}`"
          class="tree-item"
        >
          <!-- Render Group -->
          <div
            v-if="rootItem.type === 'group'"
            class="tree-group"
            :class="{
              'drag-over':
                dragOverItem?.id === rootItem.item.id &&
                dragOverType === 'group' &&
                dragOverMode === 'into',
              'drag-over-before':
                dragOverItem?.id === rootItem.item.id &&
                dragOverType === 'group' &&
                dragPosition === 'before' &&
                dragOverMode === 'reorder',
              'drag-over-after':
                dragOverItem?.id === rootItem.item.id &&
                dragOverType === 'group' &&
                dragPosition === 'after' &&
                dragOverMode === 'reorder',
            }"
            @dragover.prevent="onDragOver(rootItem.item, 'group', $event)"
            @dragenter.prevent
            @dragleave="clearDragOver"
            @drop="handleDrop(rootItem.item, 'group')"
          >
            <div class="group-header">
              <span
                class="drag-handle"
                draggable="true"
                @dragstart="startDrag(rootItem.item, 'group')"
                @dragend="endDrag"
                >⋮⋮</span
              >
              <span
                class="group-toggle"
                :class="{ collapsed: collapsedGroups[rootItem.item.id] }"
                @click="toggleGroup(rootItem.item.id)"
                >▼</span
              >
              <span class="group-name">{{ rootItem.item.name }}</span>
              <div class="group-actions">
                <button class="btn-icon" @click="editGroup(rootItem.item as Group)">✏️</button>
                <button class="btn-icon" @click="deleteGroup(rootItem.item as Group)">🗑️</button>
              </div>
            </div>

            <div v-if="!collapsedGroups[rootItem.item.id]" class="group-entries">
              <div
                v-for="entry in (rootItem.item as Group).entries.sort((a, b) => a.order - b.order)"
                :key="entry.id"
                class="tree-entry"
                :class="{
                  active: selectedEntry?.id === entry.id,
                  'drag-over-before':
                    dragOverItem?.id === entry.id &&
                    dragOverType === 'entry' &&
                    dragPosition === 'before' &&
                    dragOverMode === 'reorder',
                  'drag-over-after':
                    dragOverItem?.id === entry.id &&
                    dragOverType === 'entry' &&
                    dragPosition === 'after' &&
                    dragOverMode === 'reorder',
                }"
                draggable="true"
                @dragstart="startDrag(entry, 'entry', true)"
                @dragend="endDrag"
                @dragover.prevent="onDragOver(entry, 'entry', $event)"
                @dragenter.prevent
                @dragleave="clearDragOver"
                @drop="handleDrop(entry, 'entry', true)"
                @click="selectEntry(entry)"
              >
                <span class="drag-handle">⋮⋮</span>
                <span :class="'entry-role role-' + entry.role.toLowerCase()">
                  {{ entry.role }}
                </span>
                <span class="entry-name">{{ entry.name }}</span>
                <div
                  class="entry-toggle"
                  :class="{ enabled: entry.enabled }"
                  @click.stop="toggleEntry(entry)"
                ></div>
              </div>
            </div>
          </div>

          <!-- Render Entry -->
          <div
            v-else
            class="tree-entry"
            :class="{
              active: selectedEntry?.id === rootItem.item.id,
              'drag-over-before':
                dragOverItem?.id === rootItem.item.id &&
                dragOverType === 'entry' &&
                dragPosition === 'before' &&
                dragOverMode === 'reorder',
              'drag-over-after':
                dragOverItem?.id === rootItem.item.id &&
                dragOverType === 'entry' &&
                dragPosition === 'after' &&
                dragOverMode === 'reorder',
            }"
            draggable="true"
            @dragstart="startDrag(rootItem.item, 'entry', false)"
            @dragend="endDrag"
            @dragover.prevent="onDragOver(rootItem.item, 'entry', $event)"
            @dragenter.prevent
            @dragleave="clearDragOver"
            @drop="handleDrop(rootItem.item, 'entry', false)"
            @click="selectEntry(rootItem.item as Entry)"
          >
            <span class="drag-handle">⋮⋮</span>
            <span :class="'entry-role role-' + (rootItem.item as Entry).role.toLowerCase()">
              {{ (rootItem.item as Entry).role }}
            </span>
            <span class="entry-name">{{ rootItem.item.name }}</span>
            <div
              class="entry-toggle"
              :class="{ enabled: (rootItem.item as Entry).enabled }"
              @click.stop="toggleEntry(rootItem.item as Entry)"
            ></div>
          </div>
        </div>
      </div>
    </div>

    <!-- Content Editor -->
    <div class="content-editor">
      <template v-if="selectedEntry">
        <div class="editor-header">
          <div class="editor-title">
            <span :class="'entry-role role-' + selectedEntry.role.toLowerCase()">
              {{ selectedEntry.role }}
            </span>
            <span class="editor-entry-name">{{ selectedEntry.name }}</span>
          </div>
          <div class="editor-meta">
            <span>Order: {{ selectedEntry.order }}</span>
            <span>Status: {{ selectedEntry.enabled ? 'Enabled' : 'Disabled' }}</span>
            <span>ID: {{ selectedEntry.id.substring(0, 8) }}...</span>
          </div>
        </div>

        <div class="editor-content">
          <textarea
            class="code-textarea"
            v-model="selectedEntry.content"
            @input="contentChanged = true"
            placeholder="Enter your Weaver template content here..."
          ></textarea>
        </div>

        <div class="editor-actions">
          <button class="btn-secondary" @click="revertChanges">Revert</button>
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
  </div>

  <!-- Add Group Modal -->
  <div v-if="showAddGroupModal" class="modal-overlay" @click.self="showAddGroupModal = false">
    <div class="modal">
      <div class="modal-title">{{ editingGroupId ? 'Edit Group' : 'Add New Group' }}</div>
      <div class="form-group">
        <label class="form-label">Group Name</label>
        <input
          class="form-input"
          v-model="newGroup.name"
          placeholder="e.g., Character Information"
        />
      </div>
      <div class="form-group">
        <label class="form-label">Description (Optional)</label>
        <input
          class="form-input"
          v-model="newGroup.description"
          placeholder="Brief description of this group"
        />
      </div>
      <div class="form-group">
        <label class="form-label">Color (Optional)</label>
        <input class="form-input" v-model="newGroup.color" placeholder="#8a7fdb" />
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" @click="showAddGroupModal = false">Cancel</button>
        <button class="btn-primary" @click="addGroup">
          {{ editingGroupId ? 'Update Group' : 'Add Group' }}
        </button>
      </div>
    </div>
  </div>

  <!-- Add Entry Modal -->
  <div v-if="showAddEntryModal" class="modal-overlay" @click.self="showAddEntryModal = false">
    <div class="modal">
      <div class="modal-title">Add New Entry</div>
      <div class="form-group">
        <label class="form-label">Entry Name</label>
        <input class="form-input" v-model="newEntry.name" placeholder="e.g., System Instructions" />
      </div>
      <div class="form-group">
        <label class="form-label">Role</label>
        <select class="form-select" v-model="newEntry.role">
          <option value="System">System</option>
          <option value="Assistant">Assistant</option>
          <option value="User">User</option>
        </select>
      </div>
      <div class="form-group">
        <label class="form-label">Add to Group (Optional)</label>
        <select class="form-select" v-model="newEntry.groupId">
          <option value="">No Group</option>
          <option v-for="group in groups" :key="group.id" :value="group.id">
            {{ group.name }}
          </option>
        </select>
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" @click="showAddEntryModal = false">Cancel</button>
        <button class="btn-primary" @click="addEntry">Add Entry</button>
      </div>
    </div>
  </div>
</template>
