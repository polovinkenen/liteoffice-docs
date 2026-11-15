import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { Editor } from "@tiptap/react";
import { isTauri } from "../../lib/tauri";
import {
  confirmDiscardAll,
  confirmDiscardChanges,
  confirmQuit,
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

export function useDocumentWorkspace(): DocumentWorkspace {
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

  const markDirty = useCallback(
    (id: string) => {
      const tab = tabsRef.current.find((candidate) => candidate.id === id);

      if (!tab || tab.dirty) {
        return;
      }

      patchTab(id, { dirty: true });
    },
    [patchTab],
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
      } catch (cause) {
        patchTab(id, { error: errorMessage(cause) });
      } finally {
        patchTab(id, { busy: false });
      }
    },
    [patchTab],
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
    [commitActive, commitTabs, patchTab],
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

      await quitApp();
    } catch (cause) {
      patchTab(activeIdRef.current, { error: errorMessage(cause) });
    }
  }, [patchTab]);

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
  }, [commitTabs]);

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
