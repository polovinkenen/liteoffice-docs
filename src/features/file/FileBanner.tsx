import { ru } from "../../i18n/ru";

interface FileBannerProps {
  error: string | null;
  warnings: string[];
  onDismiss: () => void;
}

export default function FileBanner({
  error,
  warnings,
  onDismiss,
}: FileBannerProps) {
  if (!error && warnings.length === 0) {
    return null;
  }

  return (
    <div className="file-banners">
      {error ? (
        <div className="file-banner file-banner--error" role="alert">
          <div className="file-banner__content">
            <strong>{ru.file.error}:</strong> <span>{error}</span>
          </div>
          <button
            type="button"
            className="file-banner__dismiss"
            onClick={onDismiss}
          >
            {ru.file.dismiss}
          </button>
        </div>
      ) : null}

      {warnings.length > 0 ? (
        <div className="file-banner file-banner--warning" role="status">
          <div className="file-banner__content">
            <strong>{ru.file.warnings}:</strong>
            <ul>
              {warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </div>
          <button
            type="button"
            className="file-banner__dismiss"
            onClick={onDismiss}
          >
            {ru.file.dismiss}
          </button>
        </div>
      ) : null}
    </div>
  );
}
