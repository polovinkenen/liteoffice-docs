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

export interface StoredAsset {
  assetId: string;
  fileName: string;
  alt: string;
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

export function importImage(path: string): Promise<StoredAsset> {
  return invoke<StoredAsset>("import_image", { path });
}

export function loadAsset(assetId: string): Promise<string> {
  return invoke<string>("load_asset", { assetId });
}

export function setDocumentDirty(dirty: boolean): Promise<void> {
  return invoke("set_document_dirty", { dirty });
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
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
