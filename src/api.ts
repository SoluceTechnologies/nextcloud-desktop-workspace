import { invoke } from '@tauri-apps/api/core';
import type { AppState, Appearance } from './types';

export const api = {
  getState: () => invoke<AppState>('get_state'),
  takeNotice: () => invoke<string | null>('take_notice'),
  addWorkspace: (url: string) => invoke<void>('add_workspace', { url }),
  removeWorkspace: (ws: string) => invoke<void>('remove_workspace', { ws }),
  renameWorkspace: (ws: string, name: string) => invoke<void>('rename_workspace', { ws, name }),
  reorderWorkspaces: (ids: string[]) => invoke<void>('reorder_workspaces', { ids }),
  activateWorkspace: (ws: string) => invoke<void>('activate_workspace', { ws }),
  activateTab: (ws: string, tab: string) => invoke<void>('activate_tab', { ws, tab }),
  closeTab: (ws: string, tab: string) => invoke<void>('close_tab', { ws, tab }),
  reorderTabs: (ws: string, ids: string[]) => invoke<void>('reorder_tabs', { ws, ids }),
  setOverlay: (on: boolean) => invoke<void>('set_overlay', { on }),
  clearBrowsingData: (ws: string) => invoke<void>('clear_browsing_data', { ws }),
  setTheme: (theme: Appearance) => invoke<void>('set_theme', { theme }),
  setWorkspaceIcon: (ws: string, icon: string | null) => invoke<void>('set_workspace_icon', { ws, icon }),
  workspaceMenu: (ws: string) => invoke<void>('workspace_menu', { ws }),
  tabMenu: (ws: string, tab: string) => invoke<void>('tab_menu', { ws, tab }),
  appsMenu: (ws: string) => invoke<void>('apps_menu', { ws }),
  revealDownload: (path: string) => invoke<void>('reveal_download', { path }),
};
