import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { ru } from "../../i18n/ru";
import { isTauri } from "../../lib/tauri";
import {
  applyResolvedTheme,
  normalizeSettings,
  readCachedSettings,
  resolveTheme,
  systemPrefersDark,
  writeCachedSettings,
  type AppSettings,
  type ResolvedTheme,
} from "./model";
import { loadSettings, saveSettings } from "./settingsApi";

export interface SettingsController {
  settings: AppSettings;
  resolvedTheme: ResolvedTheme;
  error: string | null;
  updateSettings: (patch: Partial<AppSettings>) => void;
}

export function useSettings(): SettingsController {
  const [settings, setSettings] = useState<AppSettings>(() =>
    readCachedSettings(),
  );
  const [systemDark, setSystemDark] = useState(systemPrefersDark);
  const [error, setError] = useState<string | null>(null);
  const settingsRef = useRef(settings);
  const changedRef = useRef(false);
  const saveQueueRef = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let cancelled = false;

    void loadSettings()
      .then((loaded) => {
        if (cancelled || changedRef.current) {
          return;
        }

        settingsRef.current = loaded;
        setSettings(loaded);
        writeCachedSettings(loaded);
        setError(null);
      })
      .catch(() => {
        if (!cancelled) {
          setError(ru.settings.loadError);
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (
      typeof window === "undefined" ||
      typeof window.matchMedia !== "function"
    ) {
      return;
    }

    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (event: MediaQueryListEvent) => {
      setSystemDark(event.matches);
    };

    query.addEventListener("change", handler);

    return () => {
      query.removeEventListener("change", handler);
    };
  }, []);

  const resolvedTheme = resolveTheme(settings.theme, systemDark);

  useLayoutEffect(() => {
    applyResolvedTheme(resolvedTheme);
  }, [resolvedTheme]);

  const updateSettings = useCallback((patch: Partial<AppSettings>) => {
    changedRef.current = true;

    const next = normalizeSettings({ ...settingsRef.current, ...patch });
    settingsRef.current = next;
    setSettings(next);
    writeCachedSettings(next);

    if (!isTauri()) {
      return;
    }

    saveQueueRef.current = saveQueueRef.current
      .catch(() => undefined)
      .then(() => saveSettings(next))
      .then(() => {
        setError(null);
      })
      .catch(() => {
        setError(ru.settings.saveError);
      });
  }, []);

  return {
    settings,
    resolvedTheme,
    error,
    updateSettings,
  };
}
