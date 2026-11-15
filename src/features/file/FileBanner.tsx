import { ru } from "../../i18n/ru";
import type { FileSession } from "./useFileSession";

interface FileBannerProps {
  session: FileSession;
}

export default function FileBanner({ session }: FileBannerProps) {
  if (!session.error && session.warnings.length === 0) {
    return null;
  }

  return (
    <div className="file-banners">
      {session.error ? (
        <div className="file-banner file-banner--error" role="alert">
          <div className="file-banner__content">
            <strong>{ru.file.error}:</strong> <span>{session.error}</span>
          </div>
          <button
            type="button"
            className="file-banner__dismiss"
            onClick={session.dismissMessages}
          >
            {ru.file.dismiss}
          </button>
        </div>
      ) : null}

      {session.warnings.length > 0 ? (
        <div className="file-banner file-banner--warning" role="status">
          <div className="file-banner__content">
            <strong>{ru.file.warnings}:</strong>
            <ul>
              {session.warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </div>
          <button
            type="button"
            className="file-banner__dismiss"
            onClick={session.dismissMessages}
          >
            {ru.file.dismiss}
          </button>
        </div>
      ) : null}
    </div>
  );
}
