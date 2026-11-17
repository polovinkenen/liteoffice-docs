import type { JSONContent } from "@tiptap/core";

export type DocumentFormat = "md" | "txt" | "html" | "rtf" | "docx";

const EXTENSIONS: Record<string, DocumentFormat> = {
  md: "md",
  markdown: "md",
  txt: "txt",
  html: "html",
  htm: "html",
  rtf: "rtf",
  docx: "docx",
};

const FORMATTING_TYPES = new Set([
  "heading",
  "bulletList",
  "orderedList",
  "listItem",
  "taskList",
  "taskItem",
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

export function documentTitle(fileName: string, dirty: boolean): string {
  const name = fileName.trim() || "Без названия";
  return `WLite — ${name}${dirty ? " *" : ""}`;
}

function hasParagraphFormatting(node: JSONContent): boolean {
  if (node.type !== "paragraph" || !node.attrs) {
    return false;
  }

  const attrs = node.attrs as Record<string, unknown>;
  const align = typeof attrs.textAlign === "string" ? attrs.textAlign : "left";
  const indent = Number(attrs.indent ?? 0);
  const lineHeight = attrs.lineHeight;
  const spaceBefore = Number(attrs.spaceBefore ?? 0);
  const spaceAfter = Number(attrs.spaceAfter ?? 0);

  return (
    align !== "left" ||
    indent > 0 ||
    Boolean(lineHeight) ||
    spaceBefore > 0 ||
    spaceAfter > 0
  );
}

export function hasFormatting(node: JSONContent): boolean {
  if (node.type === "text" && (node.marks?.length ?? 0) > 0) {
    return true;
  }

  if (node.type && FORMATTING_TYPES.has(node.type)) {
    return true;
  }

  if (hasParagraphFormatting(node)) {
    return true;
  }

  return (node.content ?? []).some(hasFormatting);
}
