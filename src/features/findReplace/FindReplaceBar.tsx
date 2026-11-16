import { ru } from "../../i18n/ru";
import type { FindReplaceController } from "./useFindReplace";

interface FindReplaceBarProps {
  controller: FindReplaceController;
}

export default function FindReplaceBar({ controller }: FindReplaceBarProps) {
  const {
    query,
    replacement,
    caseSensitive,
    wholeWord,
    matches,
    activeIndex,
    status,
    close,
    setQuery,
    setReplacement,
    setCaseSensitive,
    setWholeWord,
    next,
    previous,
    replaceCurrent,
    replaceAll,
  } = controller;
  const total = matches.length;
  const current = total === 0 ? 0 : activeIndex + 1;

  return (
    <div className="find-bar" role="search" aria-label={ru.find.label}>
      <div className="find-bar__fields">
        <label className="find-bar__field">
          <span>{ru.find.findLabel}</span>
          <input
            autoFocus
            type="text"
            value={query}
            placeholder={ru.find.findPlaceholder}
            aria-label={ru.find.findLabel}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <label className="find-bar__field">
          <span>{ru.find.replaceLabel}</span>
          <input
            type="text"
            value={replacement}
            placeholder={ru.find.replacePlaceholder}
            aria-label={ru.find.replaceLabel}
            onChange={(event) => setReplacement(event.target.value)}
          />
        </label>
      </div>

      <div className="find-bar__options">
        <label className="find-bar__check">
          <input
            type="checkbox"
            checked={caseSensitive}
            onChange={(event) => setCaseSensitive(event.target.checked)}
          />
          <span>{ru.find.caseSensitive}</span>
        </label>
        <label className="find-bar__check">
          <input
            type="checkbox"
            checked={wholeWord}
            onChange={(event) => setWholeWord(event.target.checked)}
          />
          <span>{ru.find.wholeWord}</span>
        </label>
        <span className="find-bar__count" aria-live="polite">
          {total === 0
            ? ru.find.noMatches
            : ru.find.matches(current, total)}
        </span>
      </div>

      <div className="find-bar__actions">
        <button
          type="button"
          className="find-bar__button"
          title={ru.find.previous}
          aria-label={ru.find.previous}
          disabled={total === 0}
          onClick={previous}
        >
          ↑
        </button>
        <button
          type="button"
          className="find-bar__button"
          title={ru.find.next}
          aria-label={ru.find.next}
          disabled={total === 0}
          onClick={next}
        >
          ↓
        </button>
        <button
          type="button"
          className="find-bar__button"
          disabled={total === 0}
          onClick={replaceCurrent}
        >
          {ru.find.replace}
        </button>
        <button
          type="button"
          className="find-bar__button"
          disabled={total === 0}
          onClick={replaceAll}
        >
          {ru.find.replaceAll}
        </button>
        <button
          type="button"
          className="find-bar__button find-bar__button--close"
          title={ru.find.close}
          aria-label={ru.find.close}
          onClick={close}
        >
          ×
        </button>
      </div>

      {status ? (
        <p className="find-bar__status" role="status">
          {status}
        </p>
      ) : null}
    </div>
  );
}
