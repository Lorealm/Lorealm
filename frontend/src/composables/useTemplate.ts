import { computed } from 'vue'
import { useTemplateStore } from '@/stores/template'
import { useTemplatesListStore } from '@/stores/templatesList'
import { useAppStateStore } from '@/stores/appState'

/**
 * Composable for working with templates.
 * Provides a clean interface to the template stores.
 */
export function useTemplate() {
  const templateStore = useTemplateStore()
  const listStore = useTemplatesListStore()
  const appState = useAppStateStore()

  return {
    // ==================== Current Template State ====================
    
    /** The currently loaded template ID */
    templateId: computed(() => templateStore.templateId),
    
    /** The current template name */
    templateName: computed(() => templateStore.templateName),
    
    /** The current template description */
    templateDescription: computed(() => templateStore.templateDescription),
    
    /** The tree nodes of the current template */
    treeNodes: computed(() => templateStore.treeNodes),
    
    /** Available source definitions */
    availableSources: computed(() => templateStore.availableSources),
    
    /** Used source nodes as [sourceId, nodeId] tuples */
    usedSourceNodes: computed(() => templateStore.usedSourceNodes),
    
    /** Whether a template is currently loaded */
    isLoaded: computed(() => templateStore.isLoaded),
    
    /** Whether the template is currently loading */
    isLoading: computed(() => templateStore.isLoading),
    
    /** Current save state */
    saveState: computed(() => templateStore.saveState),
    
    /** Last saved timestamp */
    lastSavedAt: computed(() => templateStore.lastSavedAt),
    
    /** Current save error message */
    saveError: computed(() => templateStore.saveError),
    
    /** Whether there are unsaved changes */
    hasUnsavedChanges: computed(() => templateStore.hasUnsavedChanges),
    
    // ==================== Templates List State ====================
    
    /** All templates (sorted by updated_at) */
    templates: computed(() => listStore.sortedTemplates),
    
    /** Total template count */
    totalTemplates: computed(() => listStore.totalCount),
    
    /** Whether the list is loading */
    isListLoading: computed(() => listStore.isLoading),
    
    /** List error message */
    listError: computed(() => listStore.error),
    
    /** Whether the list has been loaded */
    hasLoadedList: computed(() => listStore.hasLoaded),
    
    // ==================== App State ====================
    
    /** The active template ID (persisted) */
    activeTemplateId: computed(() => appState.activeTemplateId),
    
    /** Whether there's an active template */
    hasActiveTemplate: computed(() => appState.hasActiveTemplate),
    
    // ==================== Template Operations ====================
    
    /** Load a template by ID */
    loadTemplate: templateStore.loadTemplate,
    
    /** Reload the current template */
    reloadTemplate: templateStore.reloadTemplate,
    
    /** Unload the current template */
    unloadTemplate: templateStore.unloadTemplate,
    
    /** Update template metadata */
    updateTemplateMetadata: templateStore.updateTemplateMetadata,
    
    // ==================== Node Operations ====================
    
    /** Add a text entry */
    addTextEntry: templateStore.addTextEntry,
    
    /** Add a group */
    addGroup: templateStore.addGroup,
    
    /** Add a source node */
    addSourceNode: templateStore.addSourceNode,
    
    /** Delete a node */
    deleteNode: templateStore.deleteNode,
    
    /** Move a node */
    moveNode: templateStore.moveNode,
    
    /** Toggle node enabled state */
    toggleNode: templateStore.toggleNode,
    
    /** Update entry content (debounced) */
    updateEntryContent: templateStore.updateEntryContent,
    
    /** Update entry title */
    updateEntryTitle: templateStore.updateEntryTitle,
    
    /** Update group */
    updateGroup: templateStore.updateGroup,
    
    /** Get node path by ID */
    getNodePath: templateStore.getNodePath,
    
    /** Find node by ID */
    findNodeById: templateStore.findNodeById,
    
    /** Find group by ID */
    findGroupById: templateStore.findGroupById,
    
    // ==================== List Operations ====================
    
    /** Fetch all templates */
    fetchTemplates: listStore.fetchTemplates,
    
    /** Create a new template */
    createTemplate: listStore.createTemplate,
    
    /** Delete a template */
    deleteTemplate: listStore.deleteTemplate,
    
    /** Get a template from the list by ID */
    getTemplateById: listStore.getTemplateById,
    
    // ==================== App State Operations ====================
    
    /** Set the active template */
    setActiveTemplate: appState.setActiveTemplate,
    
    /** Clear the active template */
    clearActiveTemplate: appState.clearActiveTemplate,
    
    /** Check if a template is the active one */
    isActiveTemplate: appState.isActiveTemplate,
  }
}

export default useTemplate