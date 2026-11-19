import { invoke } from "@tauri-apps/api/core";
import type { JSONContent } from "@tiptap/core";
import type { PdfOptions } from "./model";

export interface PdfResult {
  path: string;
  pages: number;
  bytes: number;
  warnings: string[];
}

export function exportPdf(
  document: JSONContent,
  options: PdfOptions,
  outPath: string | null,
  title: string | null,
): Promise<PdfResult> {
  return invoke<PdfResult>("export_pdf", {
    document,
    options,
    outPath,
    title,
  });
}

export function openPdfPreview(path: string): Promise<void> {
  return invoke("open_pdf_preview", { path });
}
