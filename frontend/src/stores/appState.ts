import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'

const STORAGE_KEY = 'context_app_state'

interface PersistedState {
  activeTemplateId: string | null
}

function loadPersistedState(): PersistedState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored) {
      return JSON.parse(stored)
    }
  } catch (e) {
    console.warn('Failed to load persisted app state:', e)
  }
  return { activeTemplateId: null }
}

function savePersistedState(state: PersistedState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
  } catch (e) {
    console.warn('Failed to save app state:', e)
  }
}

/**
 * Simple client-side state store for the context manager.
 *
 * This is designed to be easily replaceable with server-side state management later.
 * Current responsibilities:
 * - Track the active template ID
 * - Persist state to localStorage
 *
 * Future migration path:
 * 1. Replace localStorage with API calls to get/set user preferences
 * 2. Add WebSocket subscription for real-time state sync
 * 3. Remove this store entirely if server handles all state
 */
export const useAppStateStore = defineStore('appState', () => {
  // Load initial state from localStorage
  const persisted = loadPersistedState()

  // ==================== State ====================

  /**
   * The currently active template ID.
   * This is the template the user was last working on.
   * Used to restore their session when returning to the editor.
   */
  const activeTemplateId = ref<string | null>(persisted.activeTemplateId)

  // ==================== Computed ====================

  const hasActiveTemplate = computed(() => activeTemplateId.value !== null)

  // ==================== Actions ====================

  /**
   * Set the active template.
   * Called when:
   * - User opens a template from the list
   * - User creates a new template
   */
  function setActiveTemplate(templateId: string): void {
    activeTemplateId.value = templateId
  }

  /**
   * Clear the active template.
   * Called when:
   * - User deletes the active template
   * - User explicitly closes the template
   */
  function clearActiveTemplate(): void {
    activeTemplateId.value = null
  }

  /**
   * Check if a specific template is the active one
   */
  function isActiveTemplate(templateId: string): boolean {
    return activeTemplateId.value === templateId
  }

  // ==================== Persistence ====================

  // Watch for changes and persist to localStorage
  watch(
    activeTemplateId,
    (newValue) => {
      savePersistedState({ activeTemplateId: newValue })
    },
    { immediate: false }
  )

  // ==================== Future: Server State Migration ====================

  /**
   * Future method: Sync state from server
   * This would be called on app init to get the user's state from the backend
   */
  // async function syncFromServer(): Promise<void> {
  //   const response = await apiClient.get<PersistedState>('/user/preferences/context')
  //   activeTemplateId.value = response.activeTemplateId
  // }

  /**
   * Future method: Push state to server
   * This would replace the localStorage persistence
   */
  // async function syncToServer(): Promise<void> {
  //   await apiClient.put('/user/preferences/context', {
  //     activeTemplateId: activeTemplateId.value
  //   })
  // }

  return {
    // State
    activeTemplateId,

    // Computed
    hasActiveTemplate,

    // Actions
    setActiveTemplate,
    clearActiveTemplate,
    isActiveTemplate,
  }
})
