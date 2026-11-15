import { useState } from "react";
import { EditorContent, type Editor as TiptapEditor } from "@tiptap/react";
import { countWords } from "../lib/document";
import { ru } from "../i18n/ru";
import EditorTabs, { type EditorTab } from "./EditorTabs";
import InsertPanel from "./InsertPanel";
import TableControls from "./TableControls";
import Toolbar from "./Toolbar";

interface EditorProps {
  editor: TiptapEditor | null;
  onError: (cause: unknown) => void;
}

export default function Editor({ editor, onError }: EditorProps) {
  const [activeTab, setActiveTab] = useState<EditorTab>("home");
  const words = countWords(editor?.getText() ?? "");

  return (
    <section className="editor">
      <div className="editor-header">
        <EditorTabs active={activeTab} onChange={setActiveTab} />
        <div
          id="panel-home"
          role="tabpanel"
          aria-labelledby="tab-home"
          className="editor-panel"
          hidden={activeTab !== "home"}
        >
          <Toolbar editor={editor} />
        </div>
        <InsertPanel
          editor={editor}
          onError={onError}
          hidden={activeTab !== "insert"}
        />
        <TableControls editor={editor} />
      </div>
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
