import { useState } from "react";
import { useEditorState } from "@tiptap/react";
import FindReplaceBar from "../features/findReplace/FindReplaceBar";
import { useFindReplace } from "../features/findReplace/useFindReplace";
import StartScreen from "../features/start/StartScreen";
import DocumentTabBar from "../features/tabs/DocumentTabBar";
import type { DocumentWorkspace } from "../features/tabs/useDocumentWorkspace";
import { countWords } from "../lib/document";
import { ru } from "../i18n/ru";
import DocumentPane from "./DocumentPane";
import EditorTabs, { type EditorTab } from "./EditorTabs";
import InsertPanel from "./InsertPanel";
import TableControls from "./TableControls";
import Toolbar from "./Toolbar";

interface EditorProps {
  workspace: DocumentWorkspace;
}

export default function Editor({ workspace }: EditorProps) {
  const [activeTab, setActiveTab] = useState<EditorTab>("home");
  const editor = workspace.activeEditor;
  const showStart = workspace.activeTab.showStart;
  const find = useFindReplace(editor);
  const words =
    useEditorState({
      editor,
      selector: ({ editor: current }) =>
        current ? countWords(current.getText()) : 0,
    }) ?? 0;

  const startDocument = () => {
    workspace.dismissStartScreen();
    window.setTimeout(() => workspace.activeEditor?.commands.focus(), 0);
  };

  return (
    <section className="editor">
      <div className="editor-header">
        <DocumentTabBar
          tabs={workspace.tabs}
          activeId={workspace.activeId}
          onActivate={workspace.activateTab}
          onClose={(id) => void workspace.closeTab(id)}
          onNew={workspace.newTab}
        />
        {!showStart ? (
          <>
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
              onError={workspace.reportError}
              hidden={activeTab !== "insert"}
            />
            <TableControls editor={editor} />
            {find.open ? <FindReplaceBar controller={find} /> : null}
          </>
        ) : null}
      </div>
      <div className="document-panes" hidden={showStart}>
        {workspace.tabs.map((tab) => (
          <DocumentPane
            key={tab.id}
            tabId={tab.id}
            initialDocument={tab.initialDocument}
            active={tab.id === workspace.activeId && !showStart}
            onReady={workspace.registerEditor}
            onDispose={workspace.unregisterEditor}
            onDirty={workspace.markDirty}
          />
        ))}
      </div>
      {showStart ? (
        <StartScreen
          recent={workspace.recent}
          onNew={startDocument}
          onOpen={() => void workspace.open()}
          onOpenRecent={(path) => void workspace.openPaths([path])}
          onRemoveRecent={(path) => void workspace.removeRecent(path)}
          onClearRecent={() => void workspace.clearRecent()}
        />
      ) : null}
      <footer className="statusbar">
        <span>{ru.status.ready}</span>
        <span>
          {ru.status.words}: {words}
        </span>
      </footer>
    </section>
  );
}
