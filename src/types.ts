export type AppEntry = { id: string; name: string; href: string };
export type Tab = { id: string; appId: string; title: string; url: string; pinned: boolean };
export type Workspace = {
  id: string;
  baseUrl: string;
  name: string;
  nameCustom: boolean;
  icon: string | null;
  apps: AppEntry[];
  tabs: Tab[];
  activeTabId: string | null;
};
export type AppState = { version: number; workspaces: Workspace[]; activeWorkspaceId: string | null };
