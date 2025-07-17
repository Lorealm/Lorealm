// types/app.ts
import type { InjectionKey, Ref } from 'vue'

export interface AppState {
  messages: Ref<any[]>
  userInput: Ref<string>
  sidebarCollapsed: Ref<boolean>
  modulesPanelCollapsed: Ref<boolean>
  activeProfile: Ref<string>
  activeModule: Ref<string>
  profiles: any[]
  modules: { [key: string]: string[] }
}

export const appStateKey: InjectionKey<AppState> = Symbol('app')