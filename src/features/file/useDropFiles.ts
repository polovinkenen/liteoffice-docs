import { useEffect, useState } from "react";
import { isTauri } from "../../lib/tauri";

export function useDropFiles(onDrop: (paths: string[]) => void): boolean {
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/webview")
      .then(async ({ getCurrentWebview }) => {
        const stop = await getCurrentWebview().onDragDropEvent((event) => {
          const payload = event.payload;

          if (payload.type === "enter" || payload.type === "over") {
            setDragging(true);
            return;
          }

          if (payload.type === "leave") {
            setDragging(false);
            return;
          }

          setDragging(false);

          if (payload.paths.length > 0) {
            onDrop(payload.paths);
          }
        });

        if (disposed) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch((cause: unknown) => {
        console.error("Не удалось подключить обработку перетаскивания файлов", cause);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [onDrop]);

  return dragging;
}
