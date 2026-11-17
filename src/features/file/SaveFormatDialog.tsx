import { useEffect, useId, useState } from "react";
import { ru } from "../../i18n/ru";
import { SAVE_FORMATS } from "./formats";
import type { DocumentFormat } from "./fileUtils";

interface SaveFormatDialogProps {
  current: DocumentFormat | null;
  onConfirm: (format: DocumentFormat) => void;
  onCancel: () => void;
}

export default function SaveFormatDialog({
  current,
  onConfirm,
  onCancel,
}: SaveFormatDialogProps) {
  const [format, setFormat] = useState<DocumentFormat>(current ?? "md");
  const titleId = useId();
  const labelId = useId();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onCancel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [onCancel]);

  return (
    <div
      className="tool-modal-overlay"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
    >
      <form
        className="tool-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onSubmit={(event) => {
          event.preventDefault();
          onConfirm(format);
        }}
      >
        <h2 id={titleId}>{ru.file.saveFormatTitle}</h2>
        <div
          className="tool-field"
          role="radiogroup"
          aria-labelledby={labelId}
        >
          <span id={labelId}>{ru.file.saveFormatLabel}</span>
          <div className="save-format-list">
            {SAVE_FORMATS.map((option) => (
              <label key={option.value} className="save-format-option">
                <input
                  type="radio"
                  name="save-format"
                  value={option.value}
                  checked={format === option.value}
                  autoFocus={format === option.value}
                  onChange={() => setFormat(option.value)}
                />
                <span>{option.label}</span>
                <small>.{option.extension}</small>
              </label>
            ))}
          </div>
          {format === "txt" ? (
            <small className="save-format-hint">
              {ru.file.saveFormatTxtHint}
            </small>
          ) : null}
        </div>
        <div className="tool-modal__actions">
          <button
            type="button"
            className="tool-modal__button"
            onClick={onCancel}
          >
            {ru.file.cancel}
          </button>
          <button
            type="submit"
            className="tool-modal__button tool-modal__button--primary"
          >
            {ru.file.save}
          </button>
        </div>
      </form>
    </div>
  );
}
