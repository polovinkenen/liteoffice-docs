export interface AppInfo {
  name: string;
  version: string;
  platform: string;
  arch: string;
}

export async function fetchAppInfo(): Promise<AppInfo | null> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<AppInfo>("app_info");
  } catch {
    return null;
  }
}
