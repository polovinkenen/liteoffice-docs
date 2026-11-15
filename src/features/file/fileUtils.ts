import type { JSONContent } from "@tiptap/core";

export type DocumentFormat = "md" | "txt";

const EXTENSIONS: Record<string, DocumentFormat> = {
  md: "md",
  markdown: "md",
  txt: "txt",
};

const FORMATTING_TYPES = new Set([
  "heading",
  "bulletList",
  "orderedList",
  "listItem",
  "blockquote",
  "codeBlock",
  "table",
  "tableRow",
  "tableCell",
  "tableHeader",
  "horizontalRule",
  "image",
  "pageBreak",
]);

export function formatFromPath(path: string): DocumentFormat | null {
  const match = /\.([^.\\/]+)$/.exec(path);
  const extension = match?.[1];

  if (!extension) {
    return null;
  }

  return EXTENSIONS[extension.toLowerCase()] ?? null;
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.at(-1) ?? "";
}

export function ensureExtension(
  path: string,
  fallback: DocumentFormat,
): string {
  if (formatFromPath(path)) {
    return path;
  }

  if (/\.[^.\\/]+$/.test(path)) {
    return path;
  }

  return `${path}.${fallback}`;
}

export function documentTitle(fileName: string, dirty: boolean): string {
  const name = fileName.trim() || "Без названия";
  return `Wordlite — ${name}${dirty ? " *" : ""}`;
}

export function hasFormatting(node: JSONContent): boolean {
  if (node.type === "text" && (node.marks?.length ?? 0) > 0) {
    return true;
  }

  if (node.type && FORMATTING_TYPES.has(node.type)) {
    return true;
  }

  return (node.content ?? []).some(hasFormatting);
}
