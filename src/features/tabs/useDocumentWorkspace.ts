import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Editor } from "@tiptap/react";
import { isTauri } from "../../lib/tauri";
import {
  clearAutosaves,
  discardAutosave,
  listAutosaves,
  restoreAutosave,
  saveAutosave,
  type AutosaveRecord,
} from "../autosave/autosaveApi";
import { createDebouncer } from "../autosave/debouncer";
import { restoredTabs } from "../autosave/session";
import {
  confirmDiscardAll,
  confirmDiscardChanges,
  confirmQuit,
  confirmRestoreSession,
  confirmTxtLoss,
  pickOpenPaths,
  pickSavePath,
} from "../file/dialogs";
import {
  errorMessage,
  openDocument,
  quitApp,
  saveDocument,
  setDocumentDirty,
  type OpenedDocument,
} from "../file/fileApi";
import {
  basename,
  documentTitle,
  formatFromPath,
  hasFormatting,
} from "../file/fileUtils";
import { MENU_EVENT } from "../file/menuEvents";
import {
  anyDirty,
  createBlankTab,
  createTabId,
  findTabByPath,
  neighborAfterClose,
  nextUntitledNumber,
  type DocumentTab,
} from "./tabUtils";

export interface DocumentWorkspace {
  tabs: DocumentTab[];
  activeId: string;
  activeTab: DocumentTab;
  activeEditor: Editor | null;
  newTab: () => void;
  activateTab: (id: string) => void;
  closeTab: (id?: string) => Promise<void>;
  open: () => Promise<void>;
  save: () => Promise<void>;
  saveAs: () => Promise<void>;
  reportError: (cause: unknown) => void;
  dismissMessages: () => void;
  registerEditor: (id: string, editor: Editor) => void;
  unregisterEditor: (id: string) => void;
  markDirty: (id: string) => void;
}

const AUTOSAVE_DEBOUNCE_MS = 1500;

interface TabPatch {
  path?: string | null;
  format?: DocumentTab["format"];
  fileName?: string;
  dirty?: boolean;
  busy?: boolean;
  error?: string | null;
  warnings?: string[];
}

function tabFromOpened(opened: OpenedDocument): DocumentTab {
  return {
    id: createTabId(),
    path: opened.path,
    format: opened.format,
    fileName: basename(opened.path),
    dirty: false,
    busy: false,
    error: null,
    warnings: opened.warnings,
    initialDocument: opened.document,
    untitledNumber: null,
  };
}

export interface DocumentWorkspaceOptions {
  autosaveEnabled?: boolean;
}

export function useDocumentWorkspace(
  options: DocumentWorkspaceOptions = {},
): DocumentWorkspace {
  const autosaveEnabled = options.autosaveEnabled ?? true;
  const [initialTab] = useState(() => createBlankTab(1));
  const [tabs, setTabs] = useState<DocumentTab[]>(() => [initialTab]);
  const [activeId, setActiveId] = useState(initialTab.id);
  const [editors, setEditors] = useState<Map<string, Editor>>(
    () => new Map(),
  );

  const tabsRef = useRef(tabs);
  const activeIdRef = useRef(activeId);
  const editorsRef = useRef(editors);

  const commitTabs = useCallback((next: DocumentTab[]) => {
    tabsRef.current = next;
    setTabs(next);
  }, []);

  const commitActive = useCallback((id: string) => {
    activeIdRef.current = id;
    setActiveId(id);
  }, []);

  const patchTab = useCallback(
    (id: string, patch: TabPatch) => {
      const next = tabsRef.current.map((tab) =>
        tab.id === id ? { ...tab, ...patch } : tab,
      );
      commitTabs(next);
    },
    [commitTabs],
  );

  const registerEditor = useCallback((id: string, editor: Editor) => {
    const next = new Map(editorsRef.current);
    next.set(id, editor);
    editorsRef.current = next;
    setEditors(next);
  }, []);

  const unregisterEditor = useCallback((id: string) => {
    const next = new Map(editorsRef.current);
    next.delete(id);
    editorsRef.current = next;
    setEditors(next);
  }, []);

  const autosaveDebouncer = useRef(createDebouncer(AUTOSAVE_DEBOUNCE_MS));
  const autosaveEnabledRef = useRef(autosaveEnabled);
  const recoveryStarted = useRef(false);

  useEffect(() => {
    autosaveEnabledRef.current = autosaveEnabled;

    if (!autosaveEnabled) {
      autosaveDebouncer.current.cancelAll();
    }
  }, [autosaveEnabled]);

  const persistAutosave = useCallback(async (id: string) => {
    if (!isTauri() || !autosaveEnabledRef.current) {
      return;
    }

    const tab = tabsRef.current.find((candidate) => candidate.id === id);
    const editor = editorsRef.current.get(id);

    if (!tab || !editor || !tab.dirty) {
      return;
    }

    try {
      await saveAutosave({
        docId: id,
        path: tab.path,
        format: tab.format,
        fileName: tab.fileName,
        document: editor.getJSON(),
      });
    } catch (cause) {
      console.error("Не удалось сохранить черновик документа", cause);
    }
  }, []);

  const scheduleAutosave = useCallback(
    (id: string) => {
      if (!isTauri() || !autosaveEnabledRef.current) {
        return;
      }

      autosaveDebouncer.current.schedule(id, () => {
        void persistAutosave(id);
      });
    },
    [persistAutosave],
  );

  const flushAutosave = useCallback(
    async (id: string) => {
      autosaveDebouncer.current.cancel(id);
      await persistAutosave(id);
    },
    [persistAutosave],
  );

  const flushAllAutosaves = useCallback(async () => {
    autosaveDebouncer.current.cancelAll();
    const dirtyTabs = tabsRef.current.filter((tab) => tab.dirty);
    await Promise.all(dirtyTabs.map((tab) => persistAutosave(tab.id)));
  }, [persistAutosave]);

  const discardStoredAutosave = useCallback(async (id: string) => {
    if (!isTauri()) {
      return;
    }

    try {
      await discardAutosave(id);
    } catch (cause) {
      console.error("Не удалось удалить автосохранение", cause);
    }
  }, []);

  const markDirty = useCallback(
    (id: string) => {
      const tab = tabsRef.current.find((candidate) => candidate.id === id);

      if (!tab) {
        return;
      }

      if (!tab.dirty) {
        patchTab(id, { dirty: true });
      }

      scheduleAutosave(id);
    },
    [patchTab, scheduleAutosave],
  );

  const newTab = useCallback(() => {
    const tab = createBlankTab(nextUntitledNumber(tabsRef.current));
    commitTabs([...tabsRef.current, tab]);
    commitActive(tab.id);
  }, [commitActive, commitTabs]);

  const activateTab = useCallback(
    (id: string) => {
      if (tabsRef.current.some((tab) => tab.id === id)) {
        commitActive(id);
      }
    },
    [commitActive],
  );

  const openFiles = useCallback(
    async (paths: string[]) => {
      const current = tabsRef.current;
      const currentActive = current.find(
        (tab) => tab.id === activeIdRef.current,
      );
      const replaceBlank =
        current.length === 1 &&
        currentActive?.path === null &&
        !currentActive.dirty;
      const nextTabs = replaceBlank ? [] : [...current];
      const openedTabs: DocumentTab[] = [];
      const errors: string[] = [];

      for (const path of paths) {
        try {
          const opened = await openDocument(path);
          const existing = findTabByPath(nextTabs, opened.path);

          if (existing) {
            openedTabs.push(existing);
            continue;
          }

          const tab = tabFromOpened(opened);
          nextTabs.push(tab);
          openedTabs.push(tab);
        } catch (cause) {
          errors.push(errorMessage(cause));
        }
      }

      if (openedTabs.length > 0) {
        commitTabs(nextTabs);
        const last = openedTabs[openedTabs.length - 1]!;
        commitActive(last.id);
      }

      if (errors.length > 0) {
        const target =
          openedTabs[openedTabs.length - 1] ??
          nextTabs.find((tab) => tab.id === activeIdRef.current);

        if (target) {
          patchTab(target.id, { error: errors.join("; ") });
        }
      }
    },
    [commitActive, commitTabs, patchTab],
  );

  const open = useCallback(async () => {
    try {
      const paths = await pickOpenPaths();

      if (paths.length > 0) {
        await openFiles(paths);
      }
    } catch (cause) {
      patchTab(activeIdRef.current, { error: errorMessage(cause) });
    }
  }, [openFiles, patchTab]);

  const saveActive = useCallback(
    async (forceDialog: boolean) => {
      const id = activeIdRef.current;
      const tab = tabsRef.current.find((candidate) => candidate.id === id);
      const editor = editorsRef.current.get(id);

      if (!tab || !editor || tab.busy) {
        return;
      }

      patchTab(id, { busy: true, error: null });

      try {
        let target = tab.path;

        if (forceDialog || !target) {
          target = await pickSavePath(tab.path, tab.format);
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

        const saved = await saveDocument(target, editor.getJSON());
        patchTab(id, {
          path: saved.path,
          format: saved.format,
          fileName: basename(saved.path),
          dirty: false,
          warnings: saved.warnings,
        });
        autosaveDebouncer.current.cancel(id);
        await discardStoredAutosave(id);
      } catch (cause) {
        patchTab(id, { error: errorMessage(cause) });
      } finally {
        patchTab(id, { busy: false });
      }
    },
    [discardStoredAutosave, patchTab],
  );

  const save = useCallback(() => saveActive(false), [saveActive]);
  const saveAs = useCallback(() => saveActive(true), [saveActive]);

  const closeTab = useCallback(
    async (id?: string) => {
      const targetId = id ?? activeIdRef.current;
      const tab = tabsRef.current.find(
        (candidate) => candidate.id === targetId,
      );

      if (!tab || tab.busy) {
        return;
      }

      if (tab.dirty) {
        try {
          if (!(await confirmDiscardChanges(tab.fileName))) {
            return;
          }
        } catch (cause) {
          patchTab(targetId, { error: errorMessage(cause) });
          return;
        }
      }

      if (tab.dirty) {
        await flushAutosave(targetId);
      } else {
        autosaveDebouncer.current.cancel(targetId);
        void discardStoredAutosave(targetId);
      }

      const nextActive = neighborAfterClose(tabsRef.current, targetId);
      const remaining = tabsRef.current.filter(
        (candidate) => candidate.id !== targetId,
      );

      if (remaining.length === 0) {
        const blank = createBlankTab(nextUntitledNumber(tabsRef.current));
        commitTabs([blank]);
        commitActive(blank.id);
        return;
      }

      commitTabs(remaining);

      if (activeIdRef.current === targetId && nextActive) {
        commitActive(nextActive);
      }
    },
    [commitActive, commitTabs, discardStoredAutosave, flushAutosave, patchTab],
  );

  const reportError = useCallback(
    (cause: unknown) => {
      patchTab(activeIdRef.current, { error: errorMessage(cause) });
    },
    [patchTab],
  );

  const dismissMessages = useCallback(() => {
    patchTab(activeIdRef.current, { error: null, warnings: [] });
  }, [patchTab]);

  const handleQuit = useCallback(async () => {
    const dirtyCount = tabsRef.current.filter((tab) => tab.dirty).length;

    try {
      if (dirtyCount > 0 && !(await confirmQuit(dirtyCount))) {
        return;
      }

      await flushAllAutosaves();
      await quitApp();
    } catch (cause) {
      patchTab(activeIdRef.current, { error: errorMessage(cause) });
    }
  }, [flushAllAutosaves, patchTab]);

  const handleCloseWindow = useCallback(async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch (cause) {
      console.error(cause);
    }
  }, []);

  const activeTab = tabs.find((tab) => tab.id === activeId) ?? tabs[0]!;
  const activeEditor = editors.get(activeId) ?? null;
  const hasDirty = anyDirty(tabs);
  const activeTitle = documentTitle(activeTab.fileName, activeTab.dirty);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    void setDocumentDirty(hasDirty).catch((cause: unknown) => {
      console.error(cause);
    });
  }, [hasDirty]);

  useEffect(() => {
    document.title = activeTitle;
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().setTitle(activeTitle))
      .catch(() => undefined);
  }, [activeTitle]);

  useEffect(() => {
    if (!isTauri() || recoveryStarted.current) {
      return;
    }

    recoveryStarted.current = true;

    void (async () => {
      const entries = await listAutosaves().catch((cause: unknown) => {
        console.error("Не удалось прочитать автосохранения", cause);
        return null;
      });

      if (!entries || entries.length === 0) {
        return;
      }

      const restore = await confirmRestoreSession(entries.length).catch(
        (cause: unknown) => {
          console.error(cause);
          return false;
        },
      );

      if (!restore) {
        await clearAutosaves().catch((cause: unknown) => {
          console.error("Не удалось очистить автосохранения", cause);
        });
        return;
      }

      const records: AutosaveRecord[] = [];
      const errors: string[] = [];

      for (const entry of entries) {
        try {
          records.push(await restoreAutosave(entry.docId));
        } catch (cause) {
          errors.push(errorMessage(cause));
        }
      }

      const restored = restoredTabs(records);

      if (restored.length > 0) {
        commitTabs(restored);
        commitActive(restored[0]!.id);
      }

      if (errors.length > 0) {
        const target =
          restored[restored.length - 1]?.id ?? activeIdRef.current;
        patchTab(target, { error: errors.join("; ") });
      }
    })();
  }, [commitActive, commitTabs, patchTab]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/window")
      .then(async ({ getCurrentWindow }) => {
        const window = getCurrentWindow();
        const stop = await window.onCloseRequested(async (event) => {
          const dirtyCount = tabsRef.current.filter((tab) => tab.dirty).length;

          if (dirtyCount === 0) {
            return;
          }

          event.preventDefault();

          try {
            if (await confirmDiscardAll(dirtyCount)) {
              await flushAllAutosaves();
              const cleared = tabsRef.current.map((tab) => ({
                ...tab,
                dirty: false,
              }));
              commitTabs(cleared);
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
  }, [commitTabs, flushAllAutosaves]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    let stops: Array<() => void> = [];

    void (async () => {
      try {
        const listeners = await Promise.all([
          listen(MENU_EVENT.new, () => newTab()),
          listen(MENU_EVENT.open, () => void open()),
          listen(MENU_EVENT.save, () => void save()),
          listen(MENU_EVENT.saveAs, () => void saveAs()),
          listen(MENU_EVENT.closeDocument, () => void closeTab()),
          listen(MENU_EVENT.closeWindow, () => void handleCloseWindow()),
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
  }, [
    closeTab,
    handleCloseWindow,
    handleQuit,
    newTab,
    open,
    save,
    saveAs,
  ]);

  useEffect(() => {
    if (isTauri()) {
      return;
    }

    const handler = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey) {
        return;
      }

      const key = event.key.toLowerCase();

      if (key === "n") {
        event.preventDefault();
        newTab();
        return;
      }

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
        return;
      }

      if (key === "w" && event.shiftKey) {
        event.preventDefault();
        window.close();
        return;
      }

      if (key === "w" && !event.shiftKey) {
        event.preventDefault();
        void closeTab();
      }
    };

    window.addEventListener("keydown", handler);

    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, [closeTab, newTab, open, save, saveAs]);

  return {
    tabs,
    activeId,
    activeTab,
    activeEditor,
    newTab,
    activateTab,
    closeTab,
    open,
    save,
    saveAs,
    reportError,
    dismissMessages,
    registerEditor,
    unregisterEditor,
    markDirty,
  };
}
