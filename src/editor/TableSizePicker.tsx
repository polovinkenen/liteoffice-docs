import { useState } from "react";
import { ru } from "../i18n/ru";

const MAX_SIZE = 6;

interface TableSizePickerProps {
  onSelect: (rows: number, cols: number) => void;
}

export default function TableSizePicker({ onSelect }: TableSizePickerProps) {
  const [hover, setHover] = useState({ rows: 0, cols: 0 });

  return (
    <div
      className="table-size-picker"
      role="dialog"
      aria-label={ru.table.sizeLabel}
    >
      <div className="table-size-picker__grid">
        {Array.from({ length: MAX_SIZE }, (_, row) =>
          Array.from({ length: MAX_SIZE }, (_, col) => {
            const active = row < hover.rows && col < hover.cols;

            return (
              <button
                key={`${row}-${col}`}
                type="button"
                className={`table-size-cell${active ? " table-size-cell--active" : ""}`}
                aria-label={`${row + 1} × ${col + 1}`}
                onMouseEnter={() => setHover({ rows: row + 1, cols: col + 1 })}
                onFocus={() => setHover({ rows: row + 1, cols: col + 1 })}
                onClick={() => onSelect(row + 1, col + 1)}
              />
            );
          }),
        )}
      </div>
      <div className="table-size-picker__label">
        {hover.rows > 0
          ? `${hover.rows} × ${hover.cols}`
          : ru.table.sizeHint}
      </div>
    </div>
  );
}
