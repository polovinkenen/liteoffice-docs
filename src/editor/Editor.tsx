import { useState } from "react";
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
  const words = countWords(editor?.getText() ?? "");

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
      </div>
      <div className="document-panes">
        {workspace.tabs.map((tab) => (
          <DocumentPane
            key={tab.id}
            tabId={tab.id}
            initialDocument={tab.initialDocument}
            active={tab.id === workspace.activeId}
            onReady={workspace.registerEditor}
            onDispose={workspace.unregisterEditor}
            onDirty={workspace.markDirty}
          />
        ))}
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
