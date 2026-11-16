import { useEffect, useRef, useState } from "react";
import { GearIcon } from "../../editor/icons";
import { ru } from "../../i18n/ru";
import type { ThemePreference } from "./model";
import type { SettingsController } from "./useSettings";

const THEME_OPTIONS: Array<{ value: ThemePreference; label: string }> = [
  { value: "system", label: ru.settings.themeSystem },
  { value: "light", label: ru.settings.themeLight },
  { value: "dark", label: ru.settings.themeDark },
];

interface SettingsMenuProps {
  controller: SettingsController;
}

export default function SettingsMenu({ controller }: SettingsMenuProps) {
  const [open, setOpen] = useState(false);
  const anchorRef = useRef<HTMLDivElement>(null);
  const { settings, error, updateSettings } = controller;

  useEffect(() => {
    if (!open) {
      return;
    }

    const handlePointerDown = (event: PointerEvent) => {
      if (
        anchorRef.current &&
        !anchorRef.current.contains(event.target as Node)
      ) {
        setOpen(false);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        setOpen(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  return (
    <div className="settings-anchor" ref={anchorRef}>
      <button
        type="button"
        className={`settings-button${open ? " settings-button--active" : ""}`}
        title={ru.settings.open}
        aria-label={ru.settings.open}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <GearIcon />
      </button>

      {open ? (
        <div
          className="settings-popover"
          role="dialog"
          aria-label={ru.settings.title}
        >
          <div className="settings-popover__header">
            <h2>{ru.settings.title}</h2>
            <button
              type="button"
              className="settings-close"
              title={ru.settings.close}
              aria-label={ru.settings.close}
              onClick={() => setOpen(false)}
            >
              ×
            </button>
          </div>

          <div
            className="settings-group"
            role="radiogroup"
            aria-label={ru.settings.theme}
          >
            <span className="settings-group__label">
              {ru.settings.theme}
            </span>
            <div className="settings-segmented">
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  role="radio"
                  aria-checked={settings.theme === option.value}
                  onClick={() =>
                    updateSettings({ theme: option.value })
                  }
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div className="settings-row">
            <div className="settings-row__text">
              <span>{ru.settings.autosave}</span>
              <small>{ru.settings.autosaveHint}</small>
            </div>
            <button
              type="button"
              role="switch"
              className="settings-toggle"
              aria-label={ru.settings.autosave}
              aria-checked={settings.autosaveEnabled}
              onClick={() =>
                updateSettings({
                  autosaveEnabled: !settings.autosaveEnabled,
                })
              }
            >
              <span aria-hidden="true" />
            </button>
          </div>

          {error ? (
            <p className="settings-error" role="alert">
              {error}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
