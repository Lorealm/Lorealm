export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  contributions: {
    [pointId: string]: ContributionConfig;  // e.g., 'sidebar.profiles': { items: [...] }
  };
}

export interface ContributionConfig {
  // Generic shape; extend per point
  component?: string;  // Custom element tag
  data?: any;  // Props to pass
  handler?: (event: any) => void;  // For logic
}

export const EXTENSION_POINTS = {
  SIDEBAR_PROFILES: 'sidebar.profiles',  // Add items to profiles list
  CHAT_BUBBLE_RENDER: 'chat.bubble.render',  // Custom message renderer
  MODULES_PANEL_CONTENT: 'modules.panel.content',  // Inject into active module
  STATUS_BAR_ITEMS: 'status.bar.items',  // Add footer items
  // Add more as needed
} as const;