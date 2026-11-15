import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { countWords, createEmptyDoc } from "../lib/document";
import { ru } from "../i18n/ru";
import Toolbar from "./Toolbar";

export default function Editor() {
  const editor = useEditor({
    extensions: [StarterKit],
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

  const words = countWords(editor?.getText() ?? "");

  return (
    <section className="editor">
      <Toolbar editor={editor} />
      <div className="document-surface">
        <EditorContent editor={editor} />
      </div>
      <footer className="statusbar">
        <span>{ru.status.ready}</span>
        <span>
          {ru.status.words}: {words}
        </span>
      </footer>
    </section>
  );
}
