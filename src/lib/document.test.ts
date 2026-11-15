import { describe, expect, it } from "vitest";
import { countWords, createEmptyDoc, DOC_SCHEMA_VERSION } from "./document";

describe("createEmptyDoc", () => {
  it("создаёт документ с одним пустым абзацем", () => {
    const doc = createEmptyDoc();

    expect(doc.type).toBe("doc");
    expect(doc.content).toEqual([{ type: "paragraph" }]);
  });

  it("фиксирует версию схемы документа", () => {
    expect(DOC_SCHEMA_VERSION).toBe(1);
  });
});

describe("countWords", () => {
  it("считает слова в обычном тексте", () => {
    expect(countWords("Привет мир")).toBe(2);
  });

  it("игнорирует пустой текст и лишние пробелы", () => {
    expect(countWords("   ")).toBe(0);
    expect(countWords("один\n\nдва   три")).toBe(3);
  });
});
