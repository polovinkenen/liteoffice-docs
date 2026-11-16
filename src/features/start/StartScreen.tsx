import { ru } from "../../i18n/ru";
import type { RecentEntry } from "../recent/recentApi";
import { formatRecentDate, recentLocation } from "../recent/recentUtils";

interface StartScreenProps {
  recent: RecentEntry[];
  onNew: () => void;
  onOpen: () => void;
  onOpenRecent: (path: string) => void;
  onRemoveRecent: (path: string) => void;
  onClearRecent: () => void;
}

export default function StartScreen({
  recent,
  onNew,
  onOpen,
  onOpenRecent,
  onRemoveRecent,
  onClearRecent,
}: StartScreenProps) {
  return (
    <section className="start-screen" aria-label={ru.start.label}>
      <div className="start-screen__hero">
        <span className="start-screen__mark" aria-hidden="true">
          W
        </span>
        <h2>{ru.start.title}</h2>
        <p>{ru.start.subtitle}</p>
        <div className="start-screen__actions">
          <button
            type="button"
            className="start-button start-button--primary"
            onClick={onNew}
          >
            {ru.start.newDocument}
          </button>
          <button type="button" className="start-button" onClick={onOpen}>
            {ru.start.open}
          </button>
        </div>
      </div>

      <div className="start-screen__recent">
        <div className="start-screen__recent-header">
          <h3>{ru.start.recent}</h3>
          {recent.length > 0 ? (
            <button
              type="button"
              className="start-screen__clear"
              onClick={onClearRecent}
            >
              {ru.start.clear}
            </button>
          ) : null}
        </div>

        {recent.length === 0 ? (
          <p className="start-screen__empty">{ru.start.empty}</p>
        ) : (
          <ul className="recent-list">
            {recent.map((entry) => {
              const location = recentLocation(entry.path);
              const date = formatRecentDate(entry.openedAt);

              return (
                <li className="recent-item" key={entry.path}>
                  <button
                    type="button"
                    className="recent-item__open"
                    title={entry.path}
                    onClick={() => onOpenRecent(entry.path)}
                  >
                    <span className="recent-item__name">
                      {entry.fileName}
                    </span>
                    <span className="recent-item__meta">
                      {[date, location].filter(Boolean).join(" · ")}
                    </span>
                  </button>
                  <button
                    type="button"
                    className="recent-item__remove"
                    title={ru.start.remove}
                    aria-label={`${ru.start.remove}: ${entry.fileName}`}
                    onClick={() => onRemoveRecent(entry.path)}
                  >
                    ×
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </section>
  );
}
