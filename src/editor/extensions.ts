import { TaskItem, TaskList } from "@tiptap/extension-list";
import Highlight from "@tiptap/extension-highlight";
import Image from "@tiptap/extension-image";
import { TableKit } from "@tiptap/extension-table";
import TextAlign from "@tiptap/extension-text-align";
import { TextStyleKit } from "@tiptap/extension-text-style";
import { CharacterCount, Placeholder } from "@tiptap/extensions";
import StarterKit from "@tiptap/starter-kit";
import { ru } from "../i18n/ru";
import { countWords } from "../lib/document";
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
    TaskList,
    TaskItem.configure({
      nested: true,
    }),
    Placeholder.configure({
      placeholder: ru.editorPlaceholder,
    }),
    CharacterCount.configure({
      wordCounter: countWords,
    }),
    PageBreak,
    SearchHighlight,
  ];
}
