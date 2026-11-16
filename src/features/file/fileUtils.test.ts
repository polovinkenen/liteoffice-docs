import { describe, expect, it } from "vitest";
import {
  basename,
  documentTitle,
  ensureExtension,
  formatFromPath,
  hasFormatting,
} from "./fileUtils";

describe("formatFromPath", () => {
  it("определяет поддерживаемые расширения без учёта регистра", () => {
    expect(formatFromPath("/tmp/Заметка.MD")).toBe("md");
    expect(formatFromPath("README.markdown")).toBe("md");
    expect(formatFromPath("C:\\docs\\письмо.txt")).toBe("txt");
  });

  it("возвращает null для неизвестных файлов", () => {
    expect(formatFromPath("документ.odt")).toBeNull();
    expect(formatFromPath("без-расширения")).toBeNull();
  });
});

describe("basename", () => {
  it("извлекает имя файла из путей разных платформ", () => {
    expect(basename("/Users/me/Документ.md")).toBe("Документ.md");
    expect(basename("C:\\Users\\me\\Документ.txt")).toBe("Документ.txt");
  });
});

describe("ensureExtension", () => {
  it("добавляет расширение только когда его нет", () => {
    expect(ensureExtension("/tmp/Заметка", "md")).toBe("/tmp/Заметка.md");
    expect(ensureExtension("/tmp/Заметка.txt", "md")).toBe("/tmp/Заметка.txt");
    expect(ensureExtension("/tmp/Заметка.doc", "md")).toBe("/tmp/Заметка.doc");
  });
});

describe("documentTitle", () => {
  it("показывает имя файла и признак изменений", () => {
    expect(documentTitle("Заметка.md", false)).toBe("Wordlite — Заметка.md");
    expect(documentTitle("Заметка.md", true)).toBe("Wordlite — Заметка.md *");
    expect(documentTitle("", false)).toBe("Wordlite — Без названия");
  });
});

describe("hasFormatting", () => {
  it("распознаёт простое форматирование", () => {
    expect(
      hasFormatting({
        type: "doc",
        content: [
          {
            type: "paragraph",
            content: [{ type: "text", text: "важно", marks: [{ type: "bold" }] }],
          },
        ],
      }),
    ).toBe(true);
  });

  it("распознаёт структурные узлы и разрыв страницы", () => {
    expect(
      hasFormatting({
        type: "doc",
        content: [{ type: "heading", attrs: { level: 1 }, content: [] }],
      }),
    ).toBe(true);
    expect(
      hasFormatting({
        type: "doc",
        content: [{ type: "pageBreak" }],
      }),
    ).toBe(true);
    expect(
      hasFormatting({
        type: "doc",
        content: [
          {
            type: "taskList",
            content: [
              {
                type: "taskItem",
                attrs: { checked: false },
                content: [
                  {
                    type: "paragraph",
                    content: [{ type: "text", text: "задача" }],
                  },
                ],
              },
            ],
          },
        ],
      }),
    ).toBe(true);
  });

  it("распознаёт форматирование абзаца", () => {
    expect(
      hasFormatting({
        type: "doc",
        content: [
          {
            type: "paragraph",
            attrs: { textAlign: "center", indent: 1 },
            content: [{ type: "text", text: "абзац" }],
          },
        ],
      }),
    ).toBe(true);
  });

  it("не считает форматированием обычный текст и переносы", () => {
    expect(
      hasFormatting({
        type: "doc",
        content: [
          {
            type: "paragraph",
            content: [
              { type: "text", text: "строка" },
              { type: "hardBreak" },
              { type: "text", text: "вторая" },
            ],
          },
        ],
      }),
    ).toBe(false);
  });
});
