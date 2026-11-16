import Highlight from "@tiptap/extension-highlight";
import Image from "@tiptap/extension-image";
import { TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import { TextStyleKit } from "@tiptap/extension-text-style";
import StarterKit from "@tiptap/starter-kit";
import { PageBreak } from "./extensions/pageBreak";
import { ParagraphFormat } from "./extensions/paragraphFormat";
import { SearchHighlight } from "./extensions/searchHighlight";

export function createEditorExtensions() {
  return [
    StarterKit,
    TextStyleKit.configure({
      backgroundColor: false,
      lineHeight: false,
    }),
    TextAlign.configure({
      types: ["heading", "paragraph"],
    }),
    Highlight.configure({ multicolor: false }),
    ParagraphFormat,
    TableKit.configure({
      table: { resizable: false },
    }),
    Image.configure({
      inline: true,
      allowBase64: true,
    }),
    PageBreak,
    SearchHighlight,
  ];
}
