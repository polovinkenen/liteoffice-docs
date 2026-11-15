import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Editor } from "@tiptap/react";
import { isTauri } from "../../lib/tauri";
import {
  confirmDiscardChanges,
  confirmQuit,
  confirmTxtLoss,
  pickOpenPath,
  pickSavePath,
} from "./dialogs";
import {
  errorMessage,
  openDocument,
  quitApp,
  saveDocument,
  setDocumentDirty,
} from "./fileApi";
import {
  basename,
  documentTitle,
  formatFromPath,
  hasFormatting,
  type DocumentFormat,
} from "./fileUtils";
import { MENU_EVENT } from "./menuEvents";

export interface FileSession {
  path: string | null;
  format: DocumentFormat | null;
  fileName: string;
  title: string;
  dirty: boolean;
  busy: boolean;
  error: string | null;
  warnings: string[];
  open: () => Promise<void>;
  save: () => Promise<void>;
  saveAs: () => Promise<void>;
  reportError: (cause: unknown) => void;
  dismissMessages: () => void;
}

export function useFileSession(editor: Editor | null): FileSession {
  const [path, setPath] = useState<string | null>(null);
  const [format, setFormat] = useState<DocumentFormat | null>(null);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);

  const dirtyRef = useRef(false);
  const busyRef = useRef(false);
  const pathRef = useRef<string | null>(null);
  const formatRef = useRef<DocumentFormat | null>(null);

  const updateDirty = useCallback((value: boolean) => {
    dirtyRef.current = value;
    setDirty(value);
  }, []);

  const updateBusy = useCallback((value: boolean) => {
    busyRef.current = value;
    setBusy(value);
  }, []);

  const updatePath = useCallback((value: string | null) => {
    pathRef.current = value;
    setPath(value);
  }, []);

  const updateFormat = useCallback((value: DocumentFormat | null) => {
    formatRef.current = value;
    setFormat(value);
  }, []);

  useEffect(() => {
    if (!editor) {
      return;
    }

    const handler = () => updateDirty(true);
    editor.on("update", handler);

    return () => {
      editor.off("update", handler);
    };
  }, [editor, updateDirty]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    void setDocumentDirty(dirty).catch((cause: unknown) => {
      console.error(cause);
    });
  }, [dirty]);

  const load = useCallback(
    async (target: string) => {
      if (!editor) {
        return;
      }

      try {
        const opened = await openDocument(target);
        editor.commands.setContent(opened.document, { emitUpdate: false });
        updatePath(opened.path);
        updateFormat(opened.format);
        updateDirty(false);
        setWarnings(opened.warnings);
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    [editor, updateDirty, updateFormat, updatePath],
  );

  const open = useCallback(async () => {
    if (!editor || busyRef.current) {
      return;
    }

    updateBusy(true);
    setError(null);

    try {
      if (dirtyRef.current && !(await confirmDiscardChanges())) {
        return;
      }

      const target = await pickOpenPath();

      if (target) {
        await load(target);
      }
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      updateBusy(false);
    }
  }, [editor, load, updateBusy]);

  const saveTo = useCallback(
    async (target: string) => {
      if (!editor) {
        return;
      }

      try {
        const saved = await saveDocument(target, editor.getJSON());
        updatePath(saved.path);
        updateFormat(saved.format);
        updateDirty(false);
        setWarnings(saved.warnings);
      } catch (cause) {
        setError(errorMessage(cause));
      }
    },
    [editor, updateDirty, updateFormat, updatePath],
  );

  const save = useCallback(async () => {
    if (!editor || busyRef.current) {
      return;
    }

    updateBusy(true);
    setError(null);

    try {
      let target = pathRef.current;

      if (!target) {
        target = await pickSavePath(null, formatRef.current);
      }

      if (!target) {
        return;
      }

      if (
        formatFromPath(target) === "txt" &&
        hasFormatting(editor.getJSON()) &&
        !(await confirmTxtLoss())
      ) {
        return;
      }

      await saveTo(target);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      updateBusy(false);
    }
  }, [editor, saveTo, updateBusy]);

  const saveAs = useCallback(async () => {
    if (!editor || busyRef.current) {
      return;
    }

    updateBusy(true);
    setError(null);

    try {
      const target = await pickSavePath(pathRef.current, formatRef.current);

      if (!target) {
        return;
      }

      if (
        formatFromPath(target) === "txt" &&
        hasFormatting(editor.getJSON()) &&
        !(await confirmTxtLoss())
      ) {
        return;
      }

      await saveTo(target);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      updateBusy(false);
    }
  }, [editor, saveTo, updateBusy]);

  const handleQuit = useCallback(async () => {
    if (busyRef.current) {
      return;
    }

    try {
      if (dirtyRef.current && !(await confirmQuit())) {
        return;
      }

      await quitApp();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }, []);

  const reportError = useCallback((cause: unknown) => {
    setError(errorMessage(cause));
  }, []);

  const dismissMessages = useCallback(() => {
    setError(null);
    setWarnings([]);
  }, []);

  const fileName = path ? basename(path) : "";
  const title = useMemo(() => documentTitle(fileName, dirty), [fileName, dirty]);

  useEffect(() => {
    document.title = title;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().setTitle(title))
      .catch(() => undefined);
  }, [title]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    let stops: Array<() => void> = [];

    void (async () => {
      try {
        const listeners = await Promise.all([
          listen(MENU_EVENT.open, () => void open()),
          listen(MENU_EVENT.save, () => void save()),
          listen(MENU_EVENT.saveAs, () => void saveAs()),
          listen(MENU_EVENT.printFallback, () => {
            window.getSelection()?.removeAllRanges();
            window.print();
          }),
          listen(MENU_EVENT.quit, () => void handleQuit()),
          listen(MENU_EVENT.quitRequested, () => void handleQuit()),
        ]);

        if (disposed) {
          listeners.forEach((stop) => stop());
        } else {
          stops = listeners;
        }
      } catch (cause) {
        console.error(cause);
      }
    })();

    return () => {
      disposed = true;
      stops.forEach((stop) => stop());
    };
  }, [open, save, saveAs, handleQuit]);

  useEffect(() => {
    if (isTauri()) {
      return;
    }

    const handler = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey) {
        return;
      }

      const key = event.key.toLowerCase();

      if (key === "o") {
        event.preventDefault();
        void open();
        return;
      }

      if (key === "s" && event.shiftKey) {
        event.preventDefault();
        void saveAs();
        return;
      }

      if (key === "s" && !event.shiftKey) {
        event.preventDefault();
        void save();
      }
    };

    window.addEventListener("keydown", handler);

    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, [open, save, saveAs]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/window")
      .then(async ({ getCurrentWindow }) => {
        const window = getCurrentWindow();
        const stop = await window.onCloseRequested(async (event) => {
          if (!dirtyRef.current) {
            return;
          }

          event.preventDefault();

          try {
            if (await confirmDiscardChanges()) {
              updateDirty(false);
              await setDocumentDirty(false).catch(() => undefined);
              await window.destroy();
            }
          } catch (cause) {
            console.error(cause);
          }
        });

        if (disposed) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch(() => undefined);

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [updateDirty]);

  return {
    path,
    format,
    fileName,
    title,
    dirty,
    busy,
    error,
    warnings,
    open,
    save,
    saveAs,
    reportError,
    dismissMessages,
  };
}
