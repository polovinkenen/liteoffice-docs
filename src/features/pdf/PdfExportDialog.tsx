import { useEffect, useId, useState } from "react";
import { ru } from "../../i18n/ru";
import {
  marginPresetFor,
  marginsForPreset,
  MARGIN_PRESETS,
  type MarginPresetId,
  type PdfMargins,
  type PdfOptions,
} from "./model";
import type { PdfResult } from "./pdfApi";

interface PdfExportDialogProps {
  initial: PdfOptions;
  busy: boolean;
  error: string | null;
  warnings: string[];
  result: PdfResult | null;
  onPreview: (options: PdfOptions) => void;
  onExport: (options: PdfOptions) => void;
  onCancel: () => void;
}

export default function PdfExportDialog({
  initial,
  busy,
  error,
  warnings,
  result,
  onPreview,
  onExport,
  onCancel,
}: PdfExportDialogProps) {
  const [options, setOptions] = useState<PdfOptions>(initial);
  const titleId = useId();
  const marginsId = useId();
  const preset = marginPresetFor(options.margins);

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

  const updateMargin = (key: keyof PdfMargins, value: string) => {
    const parsed = Number(value);

    setOptions((current) => ({
      ...current,
      margins: {
        ...current.margins,
        [key]: Number.isFinite(parsed) ? parsed : 0,
      },
    }));
  };

  const updatePreset = (id: MarginPresetId) => {
    if (id === "custom") {
      return;
    }

    setOptions((current) => ({
      ...current,
      margins: marginsForPreset(id),
    }));
  };

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
        className="tool-modal pdf-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onSubmit={(event) => {
          event.preventDefault();
          onExport(options);
        }}
      >
        <h2 id={titleId}>{ru.pdf.title}</h2>

        <div className="pdf-columns">
          <label className="tool-field">
            {ru.pdf.paper}
            <select
              value={options.paper}
              onChange={(event) =>
                setOptions((current) => ({
                  ...current,
                  paper: event.target.value as PdfOptions["paper"],
                }))
              }
            >
              <option value="a4">{ru.pdf.paperA4}</option>
              <option value="letter">{ru.pdf.paperLetter}</option>
            </select>
          </label>

          <label className="tool-field">
            {ru.pdf.orientation}
            <select
              value={options.orientation}
              onChange={(event) =>
                setOptions((current) => ({
                  ...current,
                  orientation: event.target
                    .value as PdfOptions["orientation"],
                }))
              }
            >
              <option value="portrait">{ru.pdf.portrait}</option>
              <option value="landscape">{ru.pdf.landscape}</option>
            </select>
          </label>
        </div>

        <div className="tool-field" aria-labelledby={marginsId}>
          <span id={marginsId}>{ru.pdf.margins}</span>
          <select
            aria-label={ru.pdf.marginPreset}
            value={preset}
            onChange={(event) =>
              updatePreset(event.target.value as MarginPresetId)
            }
          >
            {MARGIN_PRESETS.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {marginLabel(candidate.id)}
              </option>
            ))}
          </select>
          <div className="pdf-margins">
            {(
              [
                ["top", ru.pdf.marginTop],
                ["right", ru.pdf.marginRight],
                ["bottom", ru.pdf.marginBottom],
                ["left", ru.pdf.marginLeft],
              ] as Array<[keyof PdfMargins, string]>
            ).map(([key, label]) => (
              <label key={key} className="pdf-margin">
                <span>{label}</span>
                <input
                  aria-label={label}
                  type="number"
                  min={0}
                  max={100}
                  step={1}
                  value={options.margins[key]}
                  onChange={(event) => updateMargin(key, event.target.value)}
                />
                <small>{ru.pdf.mm}</small>
              </label>
            ))}
          </div>
        </div>

        <label className="tool-field">
          {ru.pdf.header}
          <input
            type="text"
            value={options.header}
            maxLength={200}
            placeholder={ru.pdf.pageTextPlaceholder}
            onChange={(event) =>
              setOptions((current) => ({
                ...current,
                header: event.target.value,
              }))
            }
          />
        </label>

        <label className="tool-field">
          {ru.pdf.footer}
          <input
            type="text"
            value={options.footer}
            maxLength={200}
            placeholder={ru.pdf.pageTextPlaceholder}
            onChange={(event) =>
              setOptions((current) => ({
                ...current,
                footer: event.target.value,
              }))
            }
          />
        </label>

        <small className="pdf-hint">{ru.pdf.pageTextHint}</small>

        <label className="pdf-checkbox">
          <input
            type="checkbox"
            checked={options.pageNumbers}
            onChange={(event) =>
              setOptions((current) => ({
                ...current,
                pageNumbers: event.target.checked,
              }))
            }
          />
          <span>{ru.pdf.pageNumbers}</span>
        </label>

        {error ? (
          <div className="pdf-message pdf-message--error" role="alert">
            {error}
          </div>
        ) : null}

        {result ? (
          <div className="pdf-message">
            {ru.pdf.previewReady(result.pages, formatSize(result.bytes))}
          </div>
        ) : null}

        {warnings.length > 0 ? (
          <div className="pdf-message pdf-message--warning">
            <strong>{ru.file.warnings}</strong>
            <ul>
              {warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </div>
        ) : null}

        <div className="tool-modal__actions">
          <button
            type="button"
            className="tool-modal__button"
            onClick={onCancel}
            disabled={busy}
          >
            {ru.pdf.cancel}
          </button>
          <button
            type="button"
            className="tool-modal__button"
            onClick={() => onPreview(options)}
            disabled={busy}
          >
            {busy ? ru.pdf.working : ru.pdf.preview}
          </button>
          <button
            type="submit"
            className="tool-modal__button tool-modal__button--primary"
            disabled={busy}
          >
            {ru.pdf.export}
          </button>
        </div>
      </form>
    </div>
  );
}

function marginLabel(id: MarginPresetId): string {
  switch (id) {
    case "normal":
      return ru.pdf.presetNormal;
    case "narrow":
      return ru.pdf.presetNarrow;
    case "wide":
      return ru.pdf.presetWide;
    default:
      return ru.pdf.presetCustom;
  }
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} МБ`;
  }

  return `${Math.max(1, Math.round(bytes / 1024))} КБ`;
}
