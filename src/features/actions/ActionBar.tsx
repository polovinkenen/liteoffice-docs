import type { Editor } from "@tiptap/react";
import { ru } from "../../i18n/ru";
import { useInsertActions } from "./useInsertActions";

interface ActionBarProps {
  editor: Editor | null;
  onError: (cause: unknown) => void;
}

function ImageIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <circle cx="8.5" cy="9.5" r="1.5" />
      <path d="m21 16-5-5L5 20" />
    </svg>
  );
}

function TableIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M3 10h18M9 4v16M15 4v16" />
    </svg>
  );
}

export default function ActionBar({ editor, onError }: ActionBarProps) {
  const { insertImage, insertTable } = useInsertActions(editor, onError);

  return (
    <div className="action-bar" role="toolbar" aria-label="Основные действия">
      <div className="action-group">
        <span className="action-group__label">{ru.actions.insertGroup}</span>
        <div className="action-group__buttons">
          <button
            type="button"
            className="action-button"
            title={ru.actions.imageTitle}
            disabled={!editor}
            onClick={() => void insertImage()}
          >
            <ImageIcon />
            {ru.actions.image}
          </button>
          <button
            type="button"
            className="action-button"
            title={ru.actions.tableTitle}
            disabled={!editor}
            onClick={insertTable}
          >
            <TableIcon />
            {ru.actions.table}
          </button>
        </div>
      </div>
    </div>
  );
}
