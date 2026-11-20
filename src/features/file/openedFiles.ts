import { invoke } from "@tauri-apps/api/core";

export const OPEN_FILES_EVENT = "app:open-files";

export function takeOpenedPaths(): Promise<string[]> {
  return invoke<string[]>("take_opened_paths");
}
