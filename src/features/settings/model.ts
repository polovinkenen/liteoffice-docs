export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export interface AppSettings {
  theme: ThemePreference;
  autosaveEnabled: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  autosaveEnabled: true,
};

export const SETTINGS_CACHE_KEY = "liteoffice.docs.settings.v1";

const THEMES = new Set<ThemePreference>(["system", "light", "dark"]);

export function normalizeSettings(value: unknown): AppSettings {
  if (!value || typeof value !== "object") {
    return DEFAULT_SETTINGS;
  }

  const record = value as Record<string, unknown>;
  const theme =
    typeof record.theme === "string" &&
    THEMES.has(record.theme as ThemePreference)
      ? (record.theme as ThemePreference)
      : DEFAULT_SETTINGS.theme;
  const autosaveEnabled =
    typeof record.autosaveEnabled === "boolean"
      ? record.autosaveEnabled
      : DEFAULT_SETTINGS.autosaveEnabled;

  return { theme, autosaveEnabled };
}

export function resolveTheme(
  preference: ThemePreference,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (preference === "system") {
    return systemPrefersDark ? "dark" : "light";
  }

  return preference;
}

export function systemPrefersDark(): boolean {
  if (
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  ) {
    return false;
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function readCachedSettings(
  storage: Storage | undefined = typeof window !== "undefined"
    ? window.localStorage
    : undefined,
): AppSettings {
  if (!storage) {
    return DEFAULT_SETTINGS;
  }

  try {
    const raw = storage.getItem(SETTINGS_CACHE_KEY);
    return raw ? normalizeSettings(JSON.parse(raw)) : DEFAULT_SETTINGS;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function writeCachedSettings(
  settings: AppSettings,
  storage: Storage | undefined = typeof window !== "undefined"
    ? window.localStorage
    : undefined,
): void {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(SETTINGS_CACHE_KEY, JSON.stringify(settings));
  } catch {
    return;
  }
}

export function applyResolvedTheme(theme: ResolvedTheme): void {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.dataset.theme = theme;
}

export function applyInitialTheme(): void {
  applyResolvedTheme(
    resolveTheme(readCachedSettings().theme, systemPrefersDark()),
  );
}
