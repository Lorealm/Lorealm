import { apiClient } from './client'
import type {
  TemplateListResponse,
  TemplateResponse,
  TemplateSummary,
  NodeResponse,
  MoveNodeResponse,
  ToggleNodeResponse,
  SourcesResponse,
  ValidationResponse,
  PreviewResponse,
  CreateTemplateRequest,
  UpdateTemplateRequest,
  CreateNodeRequest,
  UpdateNodeRequest,
  MoveNodeRequest,
  PreviewContextRequest,
  NodePath,
  ContextTemplate,
} from '@/types/context'

const BASE_PATH = '/templates'

export const templateApi = {
  // ==================== Template CRUD ====================

  /**
   * List all templates for the current user
   */
  async listTemplates(): Promise<TemplateListResponse> {
    return apiClient.get<TemplateListResponse>(BASE_PATH)
  },

  /**
   * Get a single template with its available sources and used source nodes
   */
  async getTemplate(templateId: string): Promise<TemplateResponse> {
    return apiClient.get<TemplateResponse>(`${BASE_PATH}/${templateId}`)
  },

  /**
   * Create a new template
   */
  async createTemplate(data: CreateTemplateRequest): Promise<TemplateSummary> {
    return apiClient.post<TemplateSummary>(BASE_PATH, data)
  },

  /**
   * Update template metadata (name, description)
   * Note: This does NOT update nodes - use node endpoints for that
   */
  async updateTemplate(templateId: string, data: UpdateTemplateRequest): Promise<ContextTemplate> {
    return apiClient.put<ContextTemplate>(`${BASE_PATH}/${templateId}`, data)
  },

  /**
   * Delete a template
   */
  async deleteTemplate(templateId: string): Promise<void> {
    return apiClient.delete(`${BASE_PATH}/${templateId}`)
  },

  // ==================== Node CRUD ====================

  /**
   * Get a specific node by ID
   */
  async getNode(templateId: string, nodeId: string): Promise<NodeResponse> {
    return apiClient.get<NodeResponse>(`${BASE_PATH}/${templateId}/nodes/${nodeId}`)
  },

  /**
   * Create a new node
   */
  async createNode(templateId: string, data: CreateNodeRequest): Promise<NodeResponse> {
    return apiClient.post<NodeResponse>(`${BASE_PATH}/${templateId}/nodes`, data)
  },

  /**
   * Update an existing node by ID
   */
  async updateNode(
    templateId: string,
    nodeId: string,
    data: UpdateNodeRequest
  ): Promise<NodeResponse> {
    return apiClient.put<NodeResponse>(`${BASE_PATH}/${templateId}/nodes/${nodeId}`, data)
  },

  /**
   * Delete a node by ID
   */
  async deleteNode(templateId: string, nodeId: string): Promise<void> {
    return apiClient.delete(`${BASE_PATH}/${templateId}/nodes/${nodeId}`)
  },

  /**
   * Move a node to a new location
   */
  async moveNode(
    templateId: string,
    nodeId: string,
    data: MoveNodeRequest
  ): Promise<MoveNodeResponse> {
    return apiClient.post<MoveNodeResponse>(`${BASE_PATH}/${templateId}/nodes/${nodeId}/move`, data)
  },

  /**
   * Toggle node enabled state
   */
  async toggleNode(templateId: string, nodeId: string): Promise<ToggleNodeResponse> {
    return apiClient.post<ToggleNodeResponse>(`${BASE_PATH}/${templateId}/nodes/${nodeId}/toggle`, {})
  },

  // ==================== Sources ====================

  /**
   * Get available sources
   */
  async getSources(): Promise<SourcesResponse> {
    return apiClient.get<SourcesResponse>('/sources')
  },

  // ==================== Validation & Preview ====================

  /**
   * Validate a template
   */
  async validateTemplate(templateId: string): Promise<ValidationResponse> {
    return apiClient.post<ValidationResponse>(`${BASE_PATH}/${templateId}/validate`, {})
  },

  /**
   * Preview the compiled context
   */
  async previewTemplate(
    templateId: string,
    data: PreviewContextRequest = {}
  ): Promise<PreviewResponse> {
    return apiClient.post<PreviewResponse>(`${BASE_PATH}/${templateId}/preview`, data)
  },
}

export default templateApi