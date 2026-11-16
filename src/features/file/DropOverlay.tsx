import { ru } from "../../i18n/ru";

export default function DropOverlay() {
  return (
    <div className="drop-overlay" aria-hidden="true">
      <div className="drop-overlay__box">
        <span className="drop-overlay__icon">↓</span>
        <p>{ru.documents.dropFiles}</p>
      </div>
    </div>
  );
}
