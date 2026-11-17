import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { ru } from "../../i18n/ru";
import { isTauri } from "../../lib/tauri";
import type { DocumentFormat } from "./fileUtils";
import { extensionFor, labelFor, saveDefaultPath, withFormatExtension } from "./formats";

function ensureTauri(): void {
  if (!isTauri()) {
    throw new Error("Файловые операции доступны только в приложении WLite");
  }
}

const OPEN_FILTERS = [
  {
    name: "Документы",
    extensions: ["md", "markdown", "txt", "html", "htm", "rtf", "docx"],
  },
];

const IMAGE_FILTERS = [
  {
    name: "Изображения",
    extensions: ["png", "jpg", "jpeg", "gif", "webp"],
  },
];

export async function pickOpenPaths(): Promise<string[]> {
  ensureTauri();

  const selected = await open({
    multiple: true,
    directory: false,
    filters: OPEN_FILTERS,
  });

  if (Array.isArray(selected)) {
    return selected;
  }

  return typeof selected === "string" ? [selected] : [];
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
  currentFormat: DocumentFormat | null,
  chosenFormat: DocumentFormat,
): Promise<string | null> {
  ensureTauri();

  const selected = await save({
    defaultPath: saveDefaultPath(currentPath, currentFormat, chosenFormat),
    filters: [
      {
        name: labelFor(chosenFormat),
        extensions: [extensionFor(chosenFormat)],
      },
    ],
  });

  if (typeof selected !== "string") {
    return null;
  }

  return withFormatExtension(selected, chosenFormat);
}

export function confirmDiscardChanges(
  documentName?: string,
): Promise<boolean> {
  const message = documentName
    ? `В документе «${documentName}» есть несохранённые изменения. Продолжить без сохранения?`
    : "В документе есть несохранённые изменения. Продолжить без сохранения?";

  return ask(message, {
    title: "WLite",
    kind: "warning",
    okLabel: "Продолжить",
    cancelLabel: "Отмена",
  });
}

export function confirmDiscardAll(count: number): Promise<boolean> {
  const message =
    count > 1
      ? `Есть несохранённые изменения в ${count} документах. Закрыть без сохранения?`
      : "В документе есть несохранённые изменения. Закрыть без сохранения?";

  return ask(message, {
    title: "WLite",
    kind: "warning",
    okLabel: "Закрыть",
    cancelLabel: "Отмена",
  });
}

export function confirmQuit(count = 1): Promise<boolean> {
  const message =
    count > 1
      ? `Есть несохранённые изменения в ${count} документах. Выйти без сохранения?`
      : "В документе есть несохранённые изменения. Выйти без сохранения?";

  return ask(message, {
    title: "WLite",
    kind: "warning",
    okLabel: "Выйти",
    cancelLabel: "Отмена",
  });
}

export function confirmRestoreSession(count: number): Promise<boolean> {
  return ask(ru.autosave.restoreMessage(count), {
    title: ru.autosave.restoreTitle,
    kind: "info",
    okLabel: ru.autosave.restoreOk,
    cancelLabel: ru.autosave.restoreCancel,
  });
}

export function confirmTxtLoss(): Promise<boolean> {
  return ask(
    "Формат TXT сохраняет только текст. Форматирование, списки и таблицы будут потеряны. Сохранить?",
    {
      title: "WLite",
      kind: "warning",
      okLabel: "Сохранить",
      cancelLabel: "Отмена",
    },
  );
}
