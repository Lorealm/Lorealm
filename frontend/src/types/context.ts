// ============================================================================
// API Types - Matching Backend DTOs
// ============================================================================

// NodePath is an array of indices to address nodes in the tree (e.g., [0], [1, 2], [2, 0, 1])
export type NodePath = number[]

// ==================== Node Content Types ====================

export interface TextContent {
  type: 'text'
  content: string
  /** Optional: role hint for the LLM (system, user, assistant) */
  role?: 'system' | 'user' | 'assistant'
}

export interface SourceContent {
  type: 'source'
  /** Which source provides this node (e.g., "chat_history", "world_info") */
  source_id: string
  /** Which specific node from that source */
  node_id: string
  /** Source-specific settings */
  settings: Record<string, unknown>
}

export interface GroupContent {
  type: 'group'
  children: ContextNode[]
  /** Optional text to prepend before group contents */
  prefix?: string
  /** Optional text to append after group contents */
  suffix?: string
  /** Whether prefix/suffix should be included even if all children are disabled */
  always_include_wrappers?: boolean
}

export type NodeContent = TextContent | SourceContent | GroupContent

// ==================== Core Data Types ====================

export interface ContextNode {
  id: string
  title: string
  description: string
  enabled: boolean
  content: NodeContent
  order: number
}

export interface ContextTemplate {
  id: string
  name: string
  description: string
  nodes: ContextNode[]
  created_at: string
  updated_at: string
}

// ==================== Source Types ====================

export type SourceType = 'builtin' | 'extension'

export interface SelectOption {
  value: string
  label: string
}

export interface SettingTypeString {
  type: 'string'
  max_length?: number
}

export interface SettingTypeInteger {
  type: 'integer'
  min?: number
  max?: number
}

export interface SettingTypeFloat {
  type: 'float'
  min?: number
  max?: number
}

export interface SettingTypeBoolean {
  type: 'boolean'
}

export interface SettingTypeSelect {
  type: 'select'
  options: SelectOption[]
}

export interface SettingTypeTokenCount {
  type: 'token_count'
}

export type SettingType = 
  | SettingTypeString
  | SettingTypeInteger
  | SettingTypeFloat
  | SettingTypeBoolean
  | SettingTypeSelect
  | SettingTypeTokenCount

export interface SettingDefinition {
  key: string
  label: string
  description: string
  setting_type: SettingType
  default_value: unknown
  required: boolean
}

export interface SourceNodeDefinition {
  /** Unique ID for this node within the source */
  node_id: string
  title: string
  description: string
  /** Icon identifier (e.g., "message-square", "book", "brain") */
  icon?: string
  /** Settings this node accepts */
  settings: SettingDefinition[]
}

export interface SourceDefinition {
  /** Unique identifier for this source */
  source_id: string
  name: string
  description: string
  /** Whether this is a built-in source or from an extension */
  source_type: SourceType
  /** All nodes this source can provide */
  nodes: SourceNodeDefinition[]
}

// ==================== Request DTOs ====================

export interface CreateTemplateRequest {
  name: string
  description?: string
}

export interface UpdateTemplateRequest {
  name?: string
  description?: string
}

export interface CreateNodeRequest {
  title: string
  description?: string
  content: NodeContent
  parent_path?: NodePath
  index?: number
}

export interface UpdateNodeRequest {
  title?: string
  description?: string
  enabled?: boolean
  content?: NodeContent
}

export interface MoveNodeRequest {
  new_parent_path?: NodePath
  new_index: number
}

export interface PreviewContextRequest {
  chat_id?: string
  character_id?: string
}

// ==================== Response DTOs ====================

export interface TemplateSummary {
  id: string
  name: string
  description: string
  node_count: number
  created_at: string
  updated_at: string
}

export interface TemplateListResponse {
  templates: TemplateSummary[]
  total: number
}

export interface TemplateResponse {
  template: ContextTemplate
  available_sources: SourceDefinition[]
  used_source_nodes: [string, string][] // Array of [source_id, node_id] tuples
}

export interface NodeResponse {
  node: ContextNode
  path: NodePath
}

export interface MoveNodeResponse {
  new_path: NodePath
}

export interface ToggleNodeResponse {
  enabled: boolean
}

export interface SourcesResponse {
  sources: SourceDefinition[]
}

export interface ValidationResponse {
  valid: boolean
  errors: string[]
}

export interface PreviewSegment {
  source_type: string
  source_id?: string
  content: string
  role?: string
  token_count: number
}

export interface PreviewResponse {
  segments: PreviewSegment[]
  total_tokens: number
  combined_text: string
}

export interface ErrorResponse {
  error: string
  code: string
  details?: unknown
}

// ============================================================================
// Frontend UI Types
// ============================================================================

export type SaveState = 'saved' | 'saving' | 'unsaved' | 'error'

// For tracking pending operations
export interface PendingOperation {
  id: string
  type: 'create' | 'update' | 'delete' | 'move'
  nodePath?: NodePath
  data?: unknown
  timestamp: number
}

// ============================================================================
// Tree Node Types (for UI rendering)
// ============================================================================

// These mirror the old types but now map to the API types

export interface Entry {
  id: string
  name: string // maps to title
  role: 'User' | 'Assistant' | 'System'
  order: number
  enabled: boolean
  content: string
  locked?: boolean
  computedSource?: string
  // For source nodes
  sourceId?: string
  nodeId?: string
  settings?: Record<string, unknown>
}

export interface Group {
  id: string
  name: string // maps to title
  description?: string
  color?: string
  order: number
  enabled: boolean
  depth: number
  children: TreeNode[]
  // Group-specific fields
  prefix?: string
  suffix?: string
  alwaysIncludeWrappers?: boolean
}

export type TreeNode = { type: 'entry'; data: Entry } | { type: 'group'; data: Group }

// ============================================================================
// Conversion Utilities
// ============================================================================

function capitalizeRole(role?: string): 'User' | 'Assistant' | 'System' {
  if (!role) return 'System' // Default to System if no role
  const map: Record<string, 'User' | 'Assistant' | 'System'> = {
    user: 'User',
    assistant: 'Assistant',
    system: 'System',
  }
  return map[role.toLowerCase()] || 'System'
}

function lowercaseRole(role: string): 'user' | 'assistant' | 'system' {
  return role.toLowerCase() as 'user' | 'assistant' | 'system'
}

export function contextNodeToTreeNode(node: ContextNode, depth: number = 0): TreeNode {
  const content = node.content

  if (content.type === 'group') {
    const group: Group = {
      id: node.id,
      name: node.title,
      description: node.description,
      order: node.order,
      enabled: node.enabled,
      depth,
      children: content.children.map((child) => contextNodeToTreeNode(child, depth + 1)),
      // Store group-specific fields
      prefix: content.prefix,
      suffix: content.suffix,
      alwaysIncludeWrappers: content.always_include_wrappers,
    }
    return { type: 'group', data: group }
  } else if (content.type === 'source') {
    const entry: Entry = {
      id: node.id,
      name: node.title,
      role: 'System', // Source nodes are always system
      order: node.order,
      enabled: node.enabled,
      content: '', // Content is computed server-side
      locked: true,
      computedSource: `${content.source_id}/${content.node_id}`,
      sourceId: content.source_id,
      nodeId: content.node_id,
      settings: content.settings,
    }
    return { type: 'entry', data: entry }
  } else {
    // text content
    const entry: Entry = {
      id: node.id,
      name: node.title,
      role: capitalizeRole(content.role),
      order: node.order,
      enabled: node.enabled,
      content: content.content,
      locked: false,
    }
    return { type: 'entry', data: entry }
  }
}

export function treeNodeToNodeContent(node: TreeNode): NodeContent {
  if (node.type === 'group') {
    const group = node.data
    return {
      type: 'group',
      children: group.children.map((child) => treeNodeToContextNode(child)),
      prefix: group.prefix,
      suffix: group.suffix,
      always_include_wrappers: group.alwaysIncludeWrappers ?? false,
    }
  } else {
    const entry = node.data
    if (entry.sourceId && entry.nodeId) {
      return {
        type: 'source',
        source_id: entry.sourceId,
        node_id: entry.nodeId,
        settings: entry.settings || {},
      }
    } else {
      return {
        type: 'text',
        content: entry.content,
        role: lowercaseRole(entry.role),
      }
    }
  }
}

export function treeNodeToContextNode(node: TreeNode): ContextNode {
  return {
    id: node.data.id,
    title: node.type === 'group' ? node.data.name : node.data.name,
    description: node.type === 'group' ? node.data.description || '' : '',
    enabled: node.type === 'group' ? node.data.enabled : node.data.enabled,
    content: treeNodeToNodeContent(node),
    order: node.data.order,
  }
}

export function templateToTreeNodes(template: ContextTemplate): TreeNode[] {
  return template.nodes.map((node) => contextNodeToTreeNode(node, 0))
}

// ============================================================================
// Modal Types
// ============================================================================

export interface GroupModalState {
  show: boolean
  mode: 'create' | 'edit'
  parentGroupId: string | null
  editingId: string | null
  form: {
    name: string
    description: string
    color: string
  }
}

export interface EntryModalState {
  show: boolean
  mode: 'create' | 'edit'
  parentGroupId: string | null
  editingId: string | null
  form: {
    name: string
    role: 'User' | 'Assistant' | 'System'
    groupId: string
  }
}

// ============================================================================
// Context (for provide/inject)
// ============================================================================

export interface ContextEditorContext {
  // State
  selectedEntry: Entry | null
  collapsedGroups: Set<string>
  maxNestingDepth: number

  // Drag & Drop
  checkMove: (evt: unknown) => boolean
  onDragStart: (nodeId: string) => void
  onNodeMoved: (nodeId: string, newParentGroupId: string | null, newIndex: number) => Promise<void>

  // UI Actions
  selectEntry: (entry: Entry) => void
  toggleGroup: (groupId: string) => void
  toggleEntry: (entry: Entry) => void

  // CRUD
  openAddGroupModal: (parentGroupId: string | null) => void
  openEditGroupModal: (group: Group) => void
  deleteGroup: (group: Group) => void
  openAddEntryModal: (parentGroupId: string | null) => void
  deleteEntry: (entry: Entry) => void

  // Utilities
  findGroupById: (id: string) => Group | null
}

// ============================================================================
// Type Guards
// ============================================================================

export function isGroup(node: TreeNode): node is { type: 'group'; data: Group } {
  return node.type === 'group'
}

export function isEntry(node: TreeNode): node is { type: 'entry'; data: Entry } {
  return node.type === 'entry'
}

export function isTextContent(content: NodeContent): content is TextContent {
  return content.type === 'text'
}

export function isSourceContent(content: NodeContent): content is SourceContent {
  return content.type === 'source'
}

export function isGroupContent(content: NodeContent): content is GroupContent {
  return content.type === 'group'
}