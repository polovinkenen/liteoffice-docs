import { invoke } from "@tauri-apps/api/core";
import type { JSONContent } from "@tiptap/core";
import type { DocumentFormat } from "../file/fileUtils";

export interface AutosaveSummary {
  docId: string;
  path: string | null;
  format: DocumentFormat | null;
  fileName: string;
  updatedAt: number;
}

export interface AutosaveRecord extends AutosaveSummary {
  document: JSONContent;
}

export interface AutosaveInput {
  docId: string;
  path: string | null;
  format: DocumentFormat | null;
  fileName: string;
  document: JSONContent;
}

export function saveAutosave(input: AutosaveInput): Promise<AutosaveSummary> {
  return invoke<AutosaveSummary>("autosave_document", {
    docId: input.docId,
    path: input.path,
    format: input.format,
    fileName: input.fileName,
    document: input.document,
  });
}

export function listAutosaves(): Promise<AutosaveSummary[]> {
  return invoke<AutosaveSummary[]>("list_autosaves");
}

export function restoreAutosave(docId: string): Promise<AutosaveRecord> {
  return invoke<AutosaveRecord>("restore_autosave", { docId });
}

export function discardAutosave(docId: string): Promise<void> {
  return invoke("discard_autosave", { docId });
}

export function clearAutosaves(): Promise<void> {
  return invoke("clear_autosaves");
}
