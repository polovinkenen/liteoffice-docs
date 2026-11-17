import { ru } from "../../i18n/ru";
import type { DocumentFormat } from "./fileUtils";

export interface SaveFormatOption {
  value: DocumentFormat;
  label: string;
  extension: string;
}

export const SAVE_FORMATS: SaveFormatOption[] = [
  { value: "md", label: ru.file.formats.md, extension: "md" },
  { value: "html", label: ru.file.formats.html, extension: "html" },
  { value: "rtf", label: ru.file.formats.rtf, extension: "rtf" },
  { value: "docx", label: ru.file.formats.docx, extension: "docx" },
  { value: "txt", label: ru.file.formats.txt, extension: "txt" },
];

const KNOWN_DOCUMENT_EXTENSION = /\.(md|markdown|txt|html|htm|rtf|docx)$/i;

export function extensionFor(format: DocumentFormat): string {
  return (
    SAVE_FORMATS.find((option) => option.value === format)?.extension ??
    "md"
  );
}

export function labelFor(format: DocumentFormat): string {
  return (
    SAVE_FORMATS.find((option) => option.value === format)?.label ?? "Markdown"
  );
}

export function withFormatExtension(
  path: string,
  format: DocumentFormat,
): string {
  const extension = extensionFor(format);

  if (KNOWN_DOCUMENT_EXTENSION.test(path)) {
    return path.replace(KNOWN_DOCUMENT_EXTENSION, `.${extension}`);
  }

  return `${path}.${extension}`;
}

export function saveDefaultPath(
  currentPath: string | null,
  currentFormat: DocumentFormat | null,
  chosenFormat: DocumentFormat,
): string {
  if (currentPath && currentFormat === chosenFormat) {
    return currentPath;
  }

  if (currentPath) {
    return withFormatExtension(currentPath, chosenFormat);
  }

  return `${ru.untitled}.${extensionFor(chosenFormat)}`;
}
