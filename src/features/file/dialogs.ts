import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { ensureExtension, type DocumentFormat } from "./fileUtils";

const OPEN_FILTERS = [
  {
    name: "Текстовые документы",
    extensions: ["md", "markdown", "txt"],
  },
];

const SAVE_FILTERS = [
  {
    name: "Markdown",
    extensions: ["md"],
  },
  {
    name: "Текст",
    extensions: ["txt"],
  },
];

export async function pickOpenPath(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: OPEN_FILTERS,
  });

  return typeof selected === "string" ? selected : null;
}

export async function pickSavePath(
  currentPath: string | null,
  format: DocumentFormat | null,
): Promise<string | null> {
  const defaultPath =
    currentPath ?? (format === "txt" ? "Без названия.txt" : "Без названия.md");

  const selected = await save({
    defaultPath,
    filters: SAVE_FILTERS,
  });

  if (typeof selected !== "string") {
    return null;
  }

  return ensureExtension(selected, format ?? "md");
}

export function confirmDiscardChanges(): Promise<boolean> {
  return ask("В документе есть несохранённые изменения. Продолжить без сохранения?", {
    title: "Wordlite",
    kind: "warning",
    okLabel: "Продолжить",
    cancelLabel: "Отмена",
  });
}

export function confirmTxtLoss(): Promise<boolean> {
  return ask(
    "Формат TXT сохраняет только текст. Форматирование, списки и таблицы будут потеряны. Сохранить?",
    {
      title: "Wordlite",
      kind: "warning",
      okLabel: "Сохранить",
      cancelLabel: "Отмена",
    },
  );
}
