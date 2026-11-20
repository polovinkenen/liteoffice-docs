import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauri } from "../../lib/tauri";
import { OPEN_FILES_EVENT, takeOpenedPaths } from "./openedFiles";

export function useOpenedFiles(
  openPaths: (paths: string[]) => Promise<void>,
): void {
  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    let stop: (() => void) | undefined;

    void (async () => {
      try {
        stop = await listen<string[]>(OPEN_FILES_EVENT, (event) => {
          void openPaths(event.payload);
        });

        if (disposed) {
          stop();
          return;
        }

        const pending = await takeOpenedPaths();

        if (!disposed && pending.length > 0) {
          void openPaths(pending);
        }
      } catch (cause) {
        console.error(cause);
      }
    })();

    return () => {
      disposed = true;
      stop?.();
    };
  }, [openPaths]);
}
