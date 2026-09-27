import { invoke } from '@tauri-apps/api/core';
import type { AppState, Appearance } from './types';

export const api = {
  getState: () => invoke<AppState>('get_state'),
  takeNotice: () => invoke<string | null>('take_notice'),
  addWorkspace: (url: string) => invoke<void>('add_workspace', { url }),
  removeWorkspace: (workspaceId: string) => invoke<void>('remove_workspace', { workspaceId }),
  renameWorkspace: (workspaceId: string, name: string) => invoke<void>('rename_workspace', { workspaceId, name }),
  reorderWorkspaces: (ids: string[]) => invoke<void>('reorder_workspaces', { ids }),
  activateWorkspace: (workspaceId: string) => invoke<void>('activate_workspace', { workspaceId }),
  activateTab: (workspaceId: string, tabId: string) => invoke<void>('activate_tab', { workspaceId, tabId }),
  closeTab: (workspaceId: string, tabId: string) => invoke<void>('close_tab', { workspaceId, tabId }),
  reorderTabs: (workspaceId: string, ids: string[]) => invoke<void>('reorder_tabs', { workspaceId, ids }),
  retryTab: (tabId: string) => invoke<void>('retry_tab', { tabId }),
  setOverlay: (open: boolean) => invoke<void>('set_overlay', { open }),
  clearBrowsingData: (workspaceId: string) => invoke<void>('clear_browsing_data', { workspaceId }),
  setTheme: (theme: Appearance) => invoke<void>('set_theme', { theme }),
  setWorkspaceIcon: (workspaceId: string, icon: string | null) =>
    invoke<void>('set_workspace_icon', { workspaceId, icon }),
  workspaceMenu: (workspaceId: string) => invoke<void>('workspace_menu', { workspaceId }),
  tabMenu: (workspaceId: string, tabId: string) => invoke<void>('tab_menu', { workspaceId, tabId }),
  appsMenu: (workspaceId: string) => invoke<void>('apps_menu', { workspaceId }),
  revealDownload: (path: string) => invoke<void>('reveal_download', { path }),
};
