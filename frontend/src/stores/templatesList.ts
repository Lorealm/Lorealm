import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { templateApi } from '@/services/api/templates'
import { useAppStateStore } from './appState'
import type { TemplateSummary } from '@/types/context'

/**
 * Store for managing the list of templates.
 * Separate from the template editor store to keep concerns clean.
 */
export const useTemplatesListStore = defineStore('templatesList', () => {
  const appState = useAppStateStore()

  // ==================== State ====================

  const templates = ref<TemplateSummary[]>([])
  const totalCount = ref(0)
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const hasLoaded = ref(false)

  // ==================== Computed ====================

  const isEmpty = computed(() => templates.value.length === 0)

  const sortedTemplates = computed(() => {
    // Sort by updated_at descending (most recent first)
    return [...templates.value].sort(
      (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    )
  })

  // ==================== Actions ====================

  /**
   * Fetch all templates from the server
   */
  async function fetchTemplates(): Promise<boolean> {
    isLoading.value = true
    error.value = null

    try {
      const response = await templateApi.listTemplates()
      templates.value = response.templates
      totalCount.value = response.total
      hasLoaded.value = true
      return true
    } catch (err) {
      console.error('Failed to fetch templates:', err)
      error.value = err instanceof Error ? err.message : 'Failed to load templates'
      return false
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Create a new template
   */
  async function createTemplate(name: string, description: string = ''): Promise<string | null> {
    isLoading.value = true
    error.value = null

    try {
      const newTemplate = await templateApi.createTemplate({ name, description })

      // Add to local list
      templates.value.push(newTemplate)
      totalCount.value++

      return newTemplate.id
    } catch (err) {
      console.error('Failed to create template:', err)
      error.value = err instanceof Error ? err.message : 'Failed to create template'
      return null
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Delete a template
   */
  async function deleteTemplate(templateId: string): Promise<boolean> {
    isLoading.value = true
    error.value = null

    try {
      await templateApi.deleteTemplate(templateId)

      // Remove from local list
      templates.value = templates.value.filter((t) => t.id !== templateId)
      totalCount.value--

      // Clear active template if it was deleted
      if (appState.isActiveTemplate(templateId)) {
        appState.clearActiveTemplate()
      }

      return true
    } catch (err) {
      console.error('Failed to delete template:', err)
      error.value = err instanceof Error ? err.message : 'Failed to delete template'
      return false
    } finally {
      isLoading.value = false
    }
  }

  /**
   * Update a template in the local list (after editing)
   * Call this when returning from the editor to refresh the list item
   */
  function updateTemplateInList(templateId: string, updates: Partial<TemplateSummary>): void {
    const index = templates.value.findIndex((t) => t.id === templateId)
    if (index !== -1) {
      templates.value[index] = { ...templates.value[index], ...updates }
    }
  }

  /**
   * Refresh a single template's data from the server
   */
  async function refreshTemplate(templateId: string): Promise<boolean> {
    try {
      const response = await templateApi.getTemplate(templateId)
      const template = response.template

      updateTemplateInList(templateId, {
        name: template.name,
        description: template.description,
        node_count: template.nodes.length,
        updated_at: template.updated_at,
      })

      return true
    } catch (err) {
      console.error('Failed to refresh template:', err)
      return false
    }
  }

  /**
   * Get a template by ID from the local list
   */
  function getTemplateById(templateId: string): TemplateSummary | undefined {
    return templates.value.find((t) => t.id === templateId)
  }

  /**
   * Clear the templates list
   */
  function clear(): void {
    templates.value = []
    totalCount.value = 0
    hasLoaded.value = false
    error.value = null
  }

  return {
    // State
    templates,
    totalCount,
    isLoading,
    error,
    hasLoaded,

    // Computed
    isEmpty,
    sortedTemplates,

    // Actions
    fetchTemplates,
    createTemplate,
    deleteTemplate,
    updateTemplateInList,
    refreshTemplate,
    getTemplateById,
    clear,
  }
})
