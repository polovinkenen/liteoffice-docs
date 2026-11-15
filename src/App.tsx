import { useEffect, useState } from "react";
import Editor from "./editor/Editor";
import { ru } from "./i18n/ru";
import { fetchAppInfo, type AppInfo } from "./lib/tauri";

export default function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [bridgeChecked, setBridgeChecked] = useState(false);

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
            <p>{ru.untitled}</p>
          </div>
        </div>
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
      </header>
      <main className="workspace">
        <Editor />
      </main>
    </div>
  );
}
