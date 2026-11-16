import { invoke } from "@tauri-apps/api/core";

export interface RecentEntry {
  path: string;
  fileName: string;
  openedAt: number;
}

export function listRecent(): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("list_recent");
}

export function addRecent(path: string): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("add_recent", { path });
}

export function removeRecent(path: string): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("remove_recent", { path });
}

export function clearRecent(): Promise<RecentEntry[]> {
  return invoke<RecentEntry[]>("clear_recent");
}
