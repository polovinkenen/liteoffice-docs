import { EditorContent, type Editor as TiptapEditor } from "@tiptap/react";
import ActionBar from "../features/actions/ActionBar";
import { countWords } from "../lib/document";
import { ru } from "../i18n/ru";
import Toolbar from "./Toolbar";

interface EditorProps {
  editor: TiptapEditor | null;
  onError: (cause: unknown) => void;
}

export default function Editor({ editor, onError }: EditorProps) {
  const words = countWords(editor?.getText() ?? "");

  return (
    <section className="editor">
      <ActionBar editor={editor} onError={onError} />
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
