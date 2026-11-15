import { useCallback } from "react";
import type { Editor } from "@tiptap/react";
import { pickImagePath } from "../file/dialogs";
import { loadImage } from "../file/fileApi";

export interface InsertActions {
  insertImage: () => Promise<void>;
  insertTable: (rows: number, cols: number) => void;
  insertPageBreak: () => void;
}

export function useInsertActions(
  editor: Editor | null,
  onError: (cause: unknown) => void,
): InsertActions {
  const insertImage = useCallback(async () => {
    if (!editor) {
      return;
    }

    try {
      const path = await pickImagePath();

      if (!path) {
        return;
      }

      const image = await loadImage(path);
      editor
        .chain()
        .focus()
        .setImage({
          src: image.dataUrl,
          alt: image.alt,
          title: image.name,
        })
        .run();
    } catch (cause) {
      onError(cause);
    }
  }, [editor, onError]);

  const insertTable = useCallback(
    (rows: number, cols: number) => {
      if (!editor) {
        return;
      }

      editor
        .chain()
        .focus()
        .insertTable({ rows, cols, withHeaderRow: true })
        .run();
    },
    [editor],
  );

  const insertPageBreak = useCallback(() => {
    if (!editor) {
      return;
    }

    editor
      .chain()
      .focus()
      .insertContent([{ type: "pageBreak" }, { type: "paragraph" }])
      .run();
  }, [editor]);

  return { insertImage, insertTable, insertPageBreak };
}
