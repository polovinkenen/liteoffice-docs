import type { Editor } from "@tiptap/react";
import { ru } from "../i18n/ru";
import {
  AddColumnIcon,
  AddRowIcon,
  DeleteTableIcon,
  RemoveColumnIcon,
  RemoveRowIcon,
} from "./icons";

interface TableControlsProps {
  editor: Editor | null;
}

export default function TableControls({ editor }: TableControlsProps) {
  if (!editor || !editor.isActive("table")) {
    return null;
  }

  const canDeleteColumn = editor.can().chain().focus().deleteColumn().run();
  const canDeleteRow = editor.can().chain().focus().deleteRow().run();

  return (
    <div className="table-controls" role="toolbar" aria-label={ru.table.controls}>
      <span className="table-controls__label">{ru.table.label}</span>

      <button
        type="button"
        className="table-control"
        onClick={() => editor.chain().focus().addColumnAfter().run()}
      >
        <AddColumnIcon />
        <span>{ru.table.addColumn}</span>
      </button>
      <button
        type="button"
        className="table-control"
        disabled={!canDeleteColumn}
        onClick={() => editor.chain().focus().deleteColumn().run()}
      >
        <RemoveColumnIcon />
        <span>{ru.table.removeColumn}</span>
      </button>

      <button
        type="button"
        className="table-control"
        onClick={() => editor.chain().focus().addRowAfter().run()}
      >
        <AddRowIcon />
        <span>{ru.table.addRow}</span>
      </button>
      <button
        type="button"
        className="table-control"
        disabled={!canDeleteRow}
        onClick={() => editor.chain().focus().deleteRow().run()}
      >
        <RemoveRowIcon />
        <span>{ru.table.removeRow}</span>
      </button>

      <button
        type="button"
        className="table-control"
        onClick={() => editor.chain().focus().deleteTable().run()}
      >
        <DeleteTableIcon />
        <span>{ru.table.deleteTable}</span>
      </button>
    </div>
  );
}
