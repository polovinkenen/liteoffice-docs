import { ru } from "../../i18n/ru";
import type { DocumentTab } from "../tabs/tabUtils";
import type { AutosaveRecord } from "./autosaveApi";

export function tabFromAutosave(record: AutosaveRecord): DocumentTab {
  return {
    id: record.docId,
    path: record.path,
    format: record.format,
    fileName: record.fileName.trim() || ru.untitled,
    dirty: true,
    busy: false,
    error: null,
    warnings: [],
    initialDocument: record.document,
    untitledNumber: null,
    showStart: false,
  };
}

export function restoredTabs(records: AutosaveRecord[]): DocumentTab[] {
  const seen = new Set<string>();
  const tabs: DocumentTab[] = [];

  for (const record of records) {
    if (seen.has(record.docId)) {
      continue;
    }

    seen.add(record.docId);
    tabs.push(tabFromAutosave(record));
  }

  return tabs;
}
