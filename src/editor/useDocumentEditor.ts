import Image from "@tiptap/extension-image";
import { TableKit } from "@tiptap/extension-table";
import { useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { createEmptyDoc } from "../lib/document";

export function useDocumentEditor() {
  return useEditor({
    extensions: [
      StarterKit,
      TableKit.configure({
        table: { resizable: false },
      }),
      Image.configure({
        inline: true,
        allowBase64: true,
      }),
    ],
    content: createEmptyDoc(),
    shouldRerenderOnTransaction: true,
    editorProps: {
      attributes: {
        class: "document-body",
        spellcheck: "true",
        "aria-label": "Текст документа",
      },
    },
  });
}
