import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { isTauri } from "../../lib/tauri";
import { ensureExtension, type DocumentFormat } from "./fileUtils";

function ensureTauri(): void {
  if (!isTauri()) {
    throw new Error("Файловые операции доступны только в приложении Wordlite");
  }
}

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

const IMAGE_FILTERS = [
  {
    name: "Изображения",
    extensions: ["png", "jpg", "jpeg", "gif", "webp"],
  },
];

export async function pickOpenPath(): Promise<string | null> {
  ensureTauri();

  const selected = await open({
    multiple: false,
    directory: false,
    filters: OPEN_FILTERS,
  });

  return typeof selected === "string" ? selected : null;
}

export async function pickImagePath(): Promise<string | null> {
  ensureTauri();

  const selected = await open({
    multiple: false,
    directory: false,
    filters: IMAGE_FILTERS,
  });

  return typeof selected === "string" ? selected : null;
}

export async function pickSavePath(
  currentPath: string | null,
  format: DocumentFormat | null,
): Promise<string | null> {
  ensureTauri();

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

export function confirmQuit(): Promise<boolean> {
  return ask(
    "В документе есть несохранённые изменения. Выйти без сохранения?",
    {
      title: "Wordlite",
      kind: "warning",
      okLabel: "Выйти",
      cancelLabel: "Отмена",
    },
  );
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
