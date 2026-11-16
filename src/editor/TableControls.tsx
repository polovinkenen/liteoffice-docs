import { useEditorState } from "@tiptap/react";
import type { Editor } from "@tiptap/react";
import { ru } from "../i18n/ru";
import {
  AddColumnIcon,
  AddRowIcon,
  DeleteTableIcon,
  MergeCellsIcon,
  RemoveColumnIcon,
  RemoveRowIcon,
  SplitCellIcon,
} from "./icons";

interface TableControlsProps {
  editor: Editor | null;
}

export default function TableControls({ editor }: TableControlsProps) {
  const state = useEditorState({
    editor,
    selector: ({ editor: current }) => {
      if (!current) {
        return null;
      }

      return {
        inTable: current.isActive("table"),
        canDeleteColumn: current
          .can()
          .chain()
          .focus()
          .deleteColumn()
          .run(),
        canDeleteRow: current.can().chain().focus().deleteRow().run(),
        canMergeCells: current.can().mergeCells(),
        canSplitCell: current.can().splitCell(),
      };
    },
  });

  if (!editor || !state?.inTable) {
    return null;
  }

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
        disabled={!state.canDeleteColumn}
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
        disabled={!state.canDeleteRow}
        onClick={() => editor.chain().focus().deleteRow().run()}
      >
        <RemoveRowIcon />
        <span>{ru.table.removeRow}</span>
      </button>

      <button
        type="button"
        className="table-control"
        disabled={!state.canMergeCells}
        onClick={() => editor.chain().focus().mergeCells().run()}
      >
        <MergeCellsIcon />
        <span>{ru.table.mergeCells}</span>
      </button>
      <button
        type="button"
        className="table-control"
        disabled={!state.canSplitCell}
        onClick={() => editor.chain().focus().splitCell().run()}
      >
        <SplitCellIcon />
        <span>{ru.table.splitCell}</span>
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
