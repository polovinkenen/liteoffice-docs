import { ru } from "../i18n/ru";

export type EditorTab = "home" | "insert";

interface EditorTabsProps {
  active: EditorTab;
  onChange: (tab: EditorTab) => void;
}

export default function EditorTabs({ active, onChange }: EditorTabsProps) {
  return (
    <div className="editor-tabs" role="tablist" aria-label={ru.tabs.label}>
      <button
        type="button"
        role="tab"
        id="tab-home"
        aria-selected={active === "home"}
        aria-controls="panel-home"
        className={`editor-tab${active === "home" ? " editor-tab--active" : ""}`}
        onClick={() => onChange("home")}
      >
        {ru.tabs.home}
      </button>
      <button
        type="button"
        role="tab"
        id="tab-insert"
        aria-selected={active === "insert"}
        aria-controls="panel-insert"
        className={`editor-tab${active === "insert" ? " editor-tab--active" : ""}`}
        onClick={() => onChange("insert")}
      >
        {ru.tabs.insert}
      </button>
    </div>
  );
}
