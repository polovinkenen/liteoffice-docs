import { ru } from "../../i18n/ru";
import type { FileSession } from "./useFileSession";

interface FileActionsProps {
  session: FileSession;
  disabled: boolean;
}

export default function FileActions({ session, disabled }: FileActionsProps) {
  return (
    <div className="file-actions">
      <button
        type="button"
        className="file-button"
        title={ru.file.openTitle}
        disabled={session.busy}
        onClick={() => void session.open()}
      >
        {ru.file.open}
      </button>
      <button
        type="button"
        className="file-button file-button--primary"
        title={ru.file.saveTitle}
        disabled={disabled || session.busy}
        onClick={() => void session.save()}
      >
        {ru.file.save}
      </button>
      <button
        type="button"
        className="file-button"
        title={ru.file.saveAsTitle}
        disabled={disabled || session.busy}
        onClick={() => void session.saveAs()}
      >
        {ru.file.saveAs}
      </button>
    </div>
  );
}
