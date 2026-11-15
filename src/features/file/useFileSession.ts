import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import {
  confirmDiscardChanges,
  confirmTxtLoss,
  pickOpenPath,
  pickSavePath,
} from "./dialogs";
import { errorMessage, openDocument, saveDocument } from "./fileApi";
import {
  basename,
  documentTitle,
  formatFromPath,
  hasFormatting,
  type DocumentFormat,
} from "./fileUtils";

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
  const pathRef = useRef<string | null>(null);
  const formatRef = useRef<DocumentFormat | null>(null);

  const updateDirty = useCallback((value: boolean) => {
    dirtyRef.current = value;
    setDirty(value);
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

  const load = useCallback(
    async (target: string) => {
      if (!editor) {
        return;
      }

      setBusy(true);
      setError(null);

      try {
        const opened = await openDocument(target);
        editor.commands.setContent(opened.document, { emitUpdate: false });
        updatePath(opened.path);
        updateFormat(opened.format);
        updateDirty(false);
        setWarnings(opened.warnings);
      } catch (cause) {
        setError(errorMessage(cause));
      } finally {
        setBusy(false);
      }
    },
    [editor, updateDirty, updateFormat, updatePath],
  );

  const open = useCallback(async () => {
    if (!editor) {
      return;
    }

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
    }
  }, [editor, load]);

  const saveTo = useCallback(
    async (target: string) => {
      if (!editor) {
        return;
      }

      setBusy(true);
      setError(null);

      try {
        const saved = await saveDocument(target, editor.getJSON());
        updatePath(saved.path);
        updateFormat(saved.format);
        updateDirty(false);
        setWarnings(saved.warnings);
      } catch (cause) {
        setError(errorMessage(cause));
      } finally {
        setBusy(false);
      }
    },
    [editor, updateDirty, updateFormat, updatePath],
  );

  const save = useCallback(async () => {
    if (!editor) {
      return;
    }

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
    }
  }, [editor, saveTo]);

  const saveAs = useCallback(async () => {
    if (!editor) {
      return;
    }

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
    }
  }, [editor, saveTo]);

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
  }, []);

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
    dismissMessages,
  };
}
