import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  arch: string;
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function fetchAppInfo(): Promise<AppInfo | null> {
  try {
    return await invoke<AppInfo>("app_info");
  } catch {
    return null;
  }
}
