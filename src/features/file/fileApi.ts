import { invoke } from "@tauri-apps/api/core";
import type { JSONContent } from "@tiptap/core";
import type { DocumentFormat } from "./fileUtils";

export interface OpenedDocument {
  path: string;
  format: DocumentFormat;
  document: JSONContent;
  warnings: string[];
}

export interface SavedDocument {
  path: string;
  format: DocumentFormat;
  warnings: string[];
}

export function openDocument(path: string): Promise<OpenedDocument> {
  return invoke<OpenedDocument>("open_document", { path });
}

export function saveDocument(
  path: string,
  document: JSONContent,
): Promise<SavedDocument> {
  return invoke<SavedDocument>("save_document", { path, document });
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string" && error) {
    return error;
  }

  if (error && typeof error === "object" && "message" in error) {
    const message = (error as { message?: unknown }).message;

    if (typeof message === "string" && message) {
      return message;
    }
  }

  return "Не удалось выполнить операцию с файлом";
}
