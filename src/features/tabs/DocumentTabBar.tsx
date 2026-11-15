import { ru } from "../../i18n/ru";
import type { DocumentTab } from "./tabUtils";

interface DocumentTabBarProps {
  tabs: DocumentTab[];
  activeId: string;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
  onNew: () => void;
}

export default function DocumentTabBar({
  tabs,
  activeId,
  onActivate,
  onClose,
  onNew,
}: DocumentTabBarProps) {
  return (
    <div className="document-tabs">
      <div
        className="document-tabs__list"
        role="tablist"
        aria-label={ru.documents.label}
      >
        {tabs.map((tab) => {
          const active = tab.id === activeId;

          return (
            <div
              key={tab.id}
              className={`document-tab${active ? " document-tab--active" : ""}`}
            >
              <button
                type="button"
                role="tab"
                aria-selected={active}
                className="document-tab__label"
                title={tab.path ?? tab.fileName}
                onClick={() => onActivate(tab.id)}
              >
                <span className="document-tab__name">{tab.fileName}</span>
                {tab.dirty ? (
                  <span
                    className="document-tab__dirty"
                    aria-label={ru.documents.unsaved}
                  >
                    •
                  </span>
                ) : null}
              </button>
              <button
                type="button"
                className="document-tab__close"
                title={ru.documents.close}
                aria-label={`${ru.documents.close}: ${tab.fileName}`}
                onClick={() => onClose(tab.id)}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
      <button
        type="button"
        className="document-tabs__add"
        title={ru.documents.new}
        aria-label={ru.documents.new}
        onClick={onNew}
      >
        +
      </button>
    </div>
  );
}
