import type { JSONContent } from "@tiptap/core";
import { createEmptyDoc } from "../../lib/document";
import type { DocumentFormat } from "../file/fileUtils";

export interface DocumentTab {
  id: string;
  path: string | null;
  format: DocumentFormat | null;
  fileName: string;
  dirty: boolean;
  busy: boolean;
  error: string | null;
  warnings: string[];
  initialDocument: JSONContent;
  untitledNumber: number | null;
  showStart: boolean;
}

let fallbackCounter = 0;

export function createTabId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }

  fallbackCounter += 1;
  return `tab-${Date.now()}-${fallbackCounter}`;
}

export function createBlankTab(
  untitledNumber: number,
  showStart = false,
): DocumentTab {
  return {
    id: createTabId(),
    path: null,
    format: null,
    fileName:
      untitledNumber === 1
        ? "Без названия"
        : `Без названия ${untitledNumber}`,
    dirty: false,
    busy: false,
    error: null,
    warnings: [],
    initialDocument: createEmptyDoc(),
    untitledNumber,
    showStart,
  };
}

export function nextUntitledNumber(tabs: DocumentTab[]): number {
  return (
    tabs.reduce(
      (max, tab) => Math.max(max, tab.untitledNumber ?? 0),
      0,
    ) + 1
  );
}

export function findTabByPath(
  tabs: DocumentTab[],
  path: string,
): DocumentTab | undefined {
  return tabs.find((tab) => tab.path === path);
}

export function neighborAfterClose(
  tabs: DocumentTab[],
  closingId: string,
): string | null {
  const index = tabs.findIndex((tab) => tab.id === closingId);

  if (index === -1) {
    return null;
  }

  const remaining = tabs.filter((tab) => tab.id !== closingId);

  if (remaining.length === 0) {
    return null;
  }

  return remaining[Math.min(index, remaining.length - 1)]?.id ?? remaining[0]?.id ?? null;
}

export function anyDirty(tabs: DocumentTab[]): boolean {
  return tabs.some((tab) => tab.dirty);
}
