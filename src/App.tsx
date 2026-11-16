import { useEffect, useState } from "react";
import Editor from "./editor/Editor";
import FileBanner from "./features/file/FileBanner";
import SettingsMenu from "./features/settings/SettingsMenu";
import { useSettings } from "./features/settings/useSettings";
import { useDocumentWorkspace } from "./features/tabs/useDocumentWorkspace";
import { ru } from "./i18n/ru";
import { fetchAppInfo, type AppInfo } from "./lib/tauri";

export default function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [bridgeChecked, setBridgeChecked] = useState(false);
  const settings = useSettings();
  const workspace = useDocumentWorkspace({
    autosaveEnabled: settings.settings.autosaveEnabled,
  });
  const active = workspace.activeTab;

  useEffect(() => {
    let cancelled = false;

    void fetchAppInfo().then((info) => {
      if (cancelled) {
        return;
      }
      setAppInfo(info);
      setBridgeChecked(true);
    });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="app">
      <header className="titlebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            W
          </span>
          <div>
            <h1>{ru.appName}</h1>
            <p>
              {active.fileName}
              {active.dirty ? ` · ${ru.file.unsaved}` : ""}
            </p>
          </div>
        </div>
        <div className="titlebar-actions">
          <div className="runtime">
            {bridgeChecked && !appInfo ? (
              <span className="runtime-badge runtime-badge--warn">
                {ru.browserMode}
              </span>
            ) : null}
            {appInfo ? (
              <span className="runtime-badge">
                {ru.status.core} {appInfo.version} · {appInfo.platform}/
                {appInfo.arch}
              </span>
            ) : null}
          </div>
          <SettingsMenu controller={settings} />
        </div>
      </header>
      <main className="workspace">
        <FileBanner
          error={active.error}
          warnings={active.warnings}
          onDismiss={workspace.dismissMessages}
        />
        <Editor workspace={workspace} />
      </main>
    </div>
  );
}
