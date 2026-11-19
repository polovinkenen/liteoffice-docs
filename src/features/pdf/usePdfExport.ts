import { useCallback, useState } from "react";
import { errorMessage } from "../file/fileApi";
import { pickPdfPath } from "../file/dialogs";
import type { DocumentWorkspace } from "../tabs/useDocumentWorkspace";
import {
  readCachedPdfOptions,
  validatePdfOptions,
  writeCachedPdfOptions,
  type PdfOptions,
} from "./model";
import { exportPdf, openPdfPreview, type PdfResult } from "./pdfApi";

export interface PdfExportController {
  open: boolean;
  initialOptions: PdfOptions;
  busy: boolean;
  error: string | null;
  warnings: string[];
  result: PdfResult | null;
  show: () => void;
  close: () => void;
  preview: (options: PdfOptions) => Promise<void>;
  exportFile: (options: PdfOptions) => Promise<void>;
}

export function usePdfExport(workspace: DocumentWorkspace): PdfExportController {
  const [open, setOpen] = useState(false);
  const [initialOptions, setInitialOptions] =
    useState<PdfOptions>(readCachedPdfOptions);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [result, setResult] = useState<PdfResult | null>(null);

  const show = useCallback(() => {
    if (!workspace.activeEditor) {
      return;
    }

    setInitialOptions(readCachedPdfOptions());
    setError(null);
    setWarnings([]);
    setResult(null);
    setOpen(true);
  }, [workspace.activeEditor]);

  const close = useCallback(() => {
    if (!busy) {
      setOpen(false);
    }
  }, [busy]);

  const prepare = useCallback((options: PdfOptions) => {
    writeCachedPdfOptions(options);
    setError(null);
    setWarnings([]);
    setResult(null);
  }, []);

  const preview = useCallback(
    async (options: PdfOptions) => {
      const editor = workspace.activeEditor;

      if (!editor || busy) {
        return;
      }

      const validation = validatePdfOptions(options);

      if (validation) {
        setError(validation);
        return;
      }

      prepare(options);
      setBusy(true);

      try {
        const output = await exportPdf(
          editor.getJSON(),
          options,
          null,
          documentTitle(workspace.activeTab.fileName),
        );

        setWarnings(output.warnings);
        setResult(output);
        await openPdfPreview(output.path);
      } catch (cause) {
        setError(errorMessage(cause));
      } finally {
        setBusy(false);
      }
    },
    [busy, prepare, workspace.activeEditor, workspace.activeTab.fileName],
  );

  const exportFile = useCallback(
    async (options: PdfOptions) => {
      const editor = workspace.activeEditor;

      if (!editor || busy) {
        return;
      }

      const validation = validatePdfOptions(options);

      if (validation) {
        setError(validation);
        return;
      }

      const title = documentTitle(workspace.activeTab.fileName);
      let target: string | null;

      try {
        target = await pickPdfPath(workspace.activeTab.path, title);
      } catch (cause) {
        setError(errorMessage(cause));
        return;
      }

      if (!target) {
        return;
      }

      prepare(options);
      setBusy(true);

      try {
        const output = await exportPdf(editor.getJSON(), options, target, title);
        workspace.reportWarnings(output.warnings);
        setOpen(false);
      } catch (cause) {
        setError(errorMessage(cause));
      } finally {
        setBusy(false);
      }
    },
    [busy, prepare, workspace],
  );

  return {
    open,
    initialOptions,
    busy,
    error,
    warnings,
    result,
    show,
    close,
    preview,
    exportFile,
  };
}

function documentTitle(fileName: string): string {
  const withoutExtension = fileName.replace(/\.[^.\\/]+$/, "").trim();
  return withoutExtension || "Без названия";
}
