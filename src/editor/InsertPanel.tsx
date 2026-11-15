import { useEffect, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import { useInsertActions } from "../features/actions/useInsertActions";
import { ru } from "../i18n/ru";
import { ImageIcon, PageBreakIcon, TableIcon } from "./icons";
import TableSizePicker from "./TableSizePicker";

interface InsertPanelProps {
  editor: Editor | null;
  onError: (cause: unknown) => void;
  hidden: boolean;
}

export default function InsertPanel({
  editor,
  onError,
  hidden,
}: InsertPanelProps) {
  const { insertImage, insertTable, insertPageBreak } = useInsertActions(
    editor,
    onError,
  );
  const [pickerOpen, setPickerOpen] = useState(false);
  const anchorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!pickerOpen) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (
        anchorRef.current &&
        !anchorRef.current.contains(event.target as Node)
      ) {
        setPickerOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setPickerOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [pickerOpen]);

  return (
    <div
      id="panel-insert"
      role="tabpanel"
      aria-labelledby="tab-insert"
      className="editor-panel ribbon"
      hidden={hidden}
    >
      <button
        type="button"
        className="ribbon-button"
        title={ru.insert.imageTitle}
        disabled={!editor}
        onClick={() => void insertImage()}
      >
        <ImageIcon />
        <span>{ru.insert.image}</span>
      </button>

      <div className="ribbon-anchor" ref={anchorRef}>
        <button
          type="button"
          className="ribbon-button"
          title={ru.insert.tableTitle}
          aria-haspopup="dialog"
          aria-expanded={pickerOpen}
          disabled={!editor}
          onClick={() => setPickerOpen((open) => !open)}
        >
          <TableIcon />
          <span>{ru.insert.table}</span>
        </button>

        {pickerOpen ? (
          <TableSizePicker
            onSelect={(rows, cols) => {
              insertTable(rows, cols);
              setPickerOpen(false);
            }}
          />
        ) : null}
      </div>

      <button
        type="button"
        className="ribbon-button"
        title={ru.insert.pageBreakTitle}
        disabled={!editor}
        onClick={insertPageBreak}
      >
        <PageBreakIcon />
        <span>{ru.insert.pageBreak}</span>
      </button>
    </div>
  );
}
