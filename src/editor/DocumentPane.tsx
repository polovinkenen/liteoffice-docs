import { useEffect } from "react";
import type { JSONContent } from "@tiptap/core";
import {
  EditorContent,
  useEditor,
  type Editor as TiptapEditor,
} from "@tiptap/react";
import { createEditorExtensions } from "./extensions";

interface DocumentPaneProps {
  tabId: string;
  initialDocument: JSONContent;
  active: boolean;
  onReady: (tabId: string, editor: TiptapEditor) => void;
  onDispose: (tabId: string) => void;
  onDirty: (tabId: string) => void;
}

export default function DocumentPane({
  tabId,
  initialDocument,
  active,
  onReady,
  onDispose,
  onDirty,
}: DocumentPaneProps) {
  const editor = useEditor({
    extensions: createEditorExtensions(),
    content: initialDocument,
    shouldRerenderOnTransaction: true,
    editorProps: {
      attributes: {
        class: "document-body",
        spellcheck: "true",
        "aria-label": "Текст документа",
      },
    },
  });

  useEffect(() => {
    if (!editor) {
      return;
    }

    onReady(tabId, editor);

    return () => {
      onDispose(tabId);
    };
  }, [editor, onDispose, onReady, tabId]);

  useEffect(() => {
    if (!editor) {
      return;
    }

    const handler = () => onDirty(tabId);
    editor.on("update", handler);

    return () => {
      editor.off("update", handler);
    };
  }, [editor, onDirty, tabId]);

  return (
    <div className="document-surface" hidden={!active}>
      <EditorContent editor={editor} />
    </div>
  );
}
