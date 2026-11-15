import type { JSONContent } from "@tiptap/core";

export const DOC_SCHEMA_VERSION = 1;

export function createEmptyDoc(): JSONContent {
  return { type: "doc", content: [{ type: "paragraph" }] };
}

export function countWords(text: string): number {
  const normalized = text.trim();
  return normalized ? normalized.split(/\s+/u).length : 0;
}
