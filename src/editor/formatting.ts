import type { Editor } from "@tiptap/react";

export interface FontOption {
  label: string;
  value: string;
}

export const FONT_FAMILIES: FontOption[] = [
  { label: "Системный", value: "" },
  { label: "Arial", value: "Arial, Helvetica, sans-serif" },
  { label: "Georgia", value: "Georgia, serif" },
  {
    label: "Times New Roman",
    value: '"Times New Roman", Times, serif',
  },
  {
    label: "Courier New",
    value: '"Courier New", Courier, monospace',
  },
  { label: "Verdana", value: "Verdana, Geneva, sans-serif" },
];

export const FONT_SIZES = [
  "10pt",
  "11pt",
  "12pt",
  "13pt",
  "14pt",
  "16pt",
  "18pt",
  "20pt",
  "24pt",
  "28pt",
  "32pt",
  "36pt",
  "48pt",
];

export const LINE_HEIGHTS = ["1", "1.15", "1.5", "2"];

export const BLOCK_STYLES = [
  { label: "Обычный", value: "paragraph" },
  { label: "Заголовок 1", value: "1" },
  { label: "Заголовок 2", value: "2" },
  { label: "Заголовок 3", value: "3" },
];

export function getBlockAttributes(editor: Editor) {
  if (editor.isActive("heading")) {
    return editor.getAttributes("heading");
  }

  return editor.getAttributes("paragraph");
}

export function getBlockStyle(editor: Editor): string {
  for (const level of [1, 2, 3]) {
    if (editor.isActive("heading", { level })) {
      return String(level);
    }
  }

  return "paragraph";
}

export interface SelectionFontState {
  value: string;
  mixed: boolean;
}

export function getSelectionFontState(
  editor: Editor,
): SelectionFontState {
  const { from, to, empty } = editor.state.selection;

  if (empty) {
    const attributes = editor.getAttributes("textStyle");

    return {
      value:
        typeof attributes.fontFamily === "string"
          ? attributes.fontFamily
          : "",
      mixed: false,
    };
  }

  const fonts = new Set<string>();

  editor.state.doc.nodesBetween(from, to, (node) => {
    if (!node.isText) {
      return;
    }

    const mark = node.marks.find((item) => item.type.name === "textStyle");
    const font =
      typeof mark?.attrs.fontFamily === "string"
        ? mark.attrs.fontFamily
        : "";

    fonts.add(font);
  });

  if (fonts.size === 0) {
    return { value: "", mixed: false };
  }

  if (fonts.size > 1) {
    return { value: "", mixed: true };
  }

  return { value: [...fonts][0] ?? "", mixed: false };
}

export function normalizeLink(value: string): string | null {
  const trimmed = value.trim();

  if (!trimmed) {
    return null;
  }

  if (
    trimmed.startsWith("#") ||
    /^(https?:\/\/|mailto:|tel:)/i.test(trimmed)
  ) {
    return trimmed;
  }

  return `https://${trimmed}`;
}
