import { describe, expect, it } from "vitest";
import {
  extensionFor,
  saveDefaultPath,
  withFormatExtension,
} from "./formats";

describe("withFormatExtension", () => {
  it("заменяет известное расширение выбранным форматом", () => {
    expect(withFormatExtension("/tmp/Отчёт.md", "docx")).toBe(
      "/tmp/Отчёт.docx",
    );
    expect(withFormatExtension("README.markdown", "html")).toBe(
      "README.html",
    );
    expect(withFormatExtension("C:\\docs\\Письмо.TXT", "rtf")).toBe(
      "C:\\docs\\Письмо.rtf",
    );
  });

  it("добавляет расширение, если известного нет", () => {
    expect(withFormatExtension("/tmp/Отчёт", "html")).toBe(
      "/tmp/Отчёт.html",
    );
    expect(withFormatExtension("/tmp/Отчёт.v2", "rtf")).toBe(
      "/tmp/Отчёт.v2.rtf",
    );
  });
});

describe("saveDefaultPath", () => {
  it("сохраняет текущий путь при том же формате", () => {
    expect(saveDefaultPath("/tmp/Отчёт.md", "md", "md")).toBe(
      "/tmp/Отчёт.md",
    );
  });

  it("меняет расширение при смене формата", () => {
    expect(saveDefaultPath("/tmp/Отчёт.md", "md", "docx")).toBe(
      "/tmp/Отчёт.docx",
    );
  });

  it("строит имя для нового документа", () => {
    expect(saveDefaultPath(null, null, "rtf")).toBe("Без названия.rtf");
    expect(saveDefaultPath(null, "txt", "html")).toBe("Без названия.html");
  });
});

describe("extensionFor", () => {
  it("возвращает каноническое расширение", () => {
    expect(extensionFor("md")).toBe("md");
    expect(extensionFor("docx")).toBe("docx");
  });
});
