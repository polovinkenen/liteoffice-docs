import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import Editor from "./editor/Editor";
import DropOverlay from "./features/file/DropOverlay";
import FileBanner from "./features/file/FileBanner";
import SaveFormatDialog from "./features/file/SaveFormatDialog";
import { MENU_EVENT } from "./features/file/menuEvents";
import { useDropFiles } from "./features/file/useDropFiles";
import PdfExportDialog from "./features/pdf/PdfExportDialog";
import { usePdfExport } from "./features/pdf/usePdfExport";
import SettingsMenu from "./features/settings/SettingsMenu";
import { useSettings } from "./features/settings/useSettings";
import { useDocumentWorkspace } from "./features/tabs/useDocumentWorkspace";
import { ru } from "./i18n/ru";
import { fetchAppInfo, isTauri, type AppInfo } from "./lib/tauri";

export default function App() {
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [bridgeChecked, setBridgeChecked] = useState(false);
  const settings = useSettings();
  const workspace = useDocumentWorkspace({
    autosaveEnabled: settings.settings.autosaveEnabled,
  });
  const dragging = useDropFiles(workspace.openPaths);
  const pdf = usePdfExport(workspace);
  const showPdfExport = pdf.show;
  const active = workspace.activeTab;

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let stop: (() => void) | undefined;
    let disposed = false;

    void listen(MENU_EVENT.exportPdf, () => showPdfExport())
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          stop = unlisten;
        }
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      stop?.();
    };
  }, [showPdfExport]);

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
      {workspace.saveFormatPrompt ? (
        <SaveFormatDialog
          current={workspace.saveFormatPrompt.current}
          onConfirm={workspace.confirmSaveFormat}
          onCancel={workspace.cancelSaveFormat}
        />
      ) : null}
      {pdf.open ? (
        <PdfExportDialog
          initial={pdf.initialOptions}
          busy={pdf.busy}
          error={pdf.error}
          warnings={pdf.warnings}
          result={pdf.result}
          onPreview={(options) => void pdf.preview(options)}
          onExport={(options) => void pdf.exportFile(options)}
          onCancel={pdf.close}
        />
      ) : null}
      {dragging ? <DropOverlay /> : null}
    </div>
  );
}
