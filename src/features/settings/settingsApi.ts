import { invoke } from "@tauri-apps/api/core";
import { normalizeSettings, type AppSettings } from "./model";

export async function loadSettings(): Promise<AppSettings> {
  const value = await invoke<unknown>("load_settings");
  return normalizeSettings(value);
}

export async function saveSettings(
  settings: AppSettings,
): Promise<AppSettings> {
  const value = await invoke<unknown>("save_settings", { settings });
  return normalizeSettings(value);
}
