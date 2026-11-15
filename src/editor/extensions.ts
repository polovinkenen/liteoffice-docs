import Image from "@tiptap/extension-image";
import { TableKit } from "@tiptap/extension-table";
import StarterKit from "@tiptap/starter-kit";
import { PageBreak } from "./extensions/pageBreak";

export function createEditorExtensions() {
  return [
    StarterKit,
    TableKit.configure({
      table: { resizable: false },
    }),
    Image.configure({
      inline: true,
      allowBase64: true,
    }),
    PageBreak,
  ];
}
