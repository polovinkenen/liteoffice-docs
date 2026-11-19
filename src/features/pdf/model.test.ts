import { describe, expect, it } from "vitest";
import {
  DEFAULT_PDF_OPTIONS,
  marginPresetFor,
  marginsForPreset,
  normalizePdfOptions,
  paperDimensionsMm,
  readCachedPdfOptions,
  validatePdfOptions,
  writeCachedPdfOptions,
} from "./model";

function storage(): Storage {
  const map = new Map<string, string>();
  return {
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (key: string) => map.get(key) ?? null,
    key: (index: number) => Array.from(map.keys())[index] ?? null,
    removeItem: (key: string) => {
      map.delete(key);
    },
    setItem: (key: string, value: string) => {
      map.set(key, value);
    },
  };
}

describe("PdfOptions model", () => {
  it("даёт дефолт A4, портрет, поля 20 мм и нумерацию", () => {
    expect(DEFAULT_PDF_OPTIONS).toMatchObject({
      paper: "a4",
      orientation: "portrait",
      margins: { top: 20, right: 20, bottom: 20, left: 20 },
      header: "",
      footer: "",
      pageNumbers: true,
    });
  });

  it("меняет размеры страницы для ориентации", () => {
    expect(paperDimensionsMm("a4", "portrait")).toEqual([210, 297]);
    expect(paperDimensionsMm("a4", "landscape")).toEqual([297, 210]);
    expect(paperDimensionsMm("letter", "landscape")).toEqual([279.4, 215.9]);
  });

  it("определяет пресеты полей", () => {
    expect(marginPresetFor(marginsForPreset("narrow"))).toBe("narrow");
    expect(marginPresetFor(marginsForPreset("wide"))).toBe("wide");
    expect(
      marginPresetFor({ top: 10, right: 20, bottom: 20, left: 20 }),
    ).toBe("custom");
  });

  it("нормализует повреждённые параметры", () => {
    expect(normalizePdfOptions(null)).toEqual(DEFAULT_PDF_OPTIONS);
    expect(
      normalizePdfOptions({
        paper: "a3",
        orientation: "sideways",
        margins: { top: "10" },
        header: 42,
        pageNumbers: "yes",
      }),
    ).toEqual(DEFAULT_PDF_OPTIONS);
  });

  it("валидирует поля и колонтитулы", () => {
    expect(validatePdfOptions(DEFAULT_PDF_OPTIONS)).toBeNull();
    expect(
      validatePdfOptions({
        ...DEFAULT_PDF_OPTIONS,
        margins: { top: 120, right: 20, bottom: 20, left: 20 },
      }),
    ).toContain("Верхнее поле");
    expect(
      validatePdfOptions({
        ...DEFAULT_PDF_OPTIONS,
        margins: { top: 20, right: 100, bottom: 20, left: 100 },
      }),
    ).toContain("Боковые поля");
    expect(
      validatePdfOptions({
        ...DEFAULT_PDF_OPTIONS,
        header: "x".repeat(201),
      }),
    ).toContain("длиннее");
    expect(
      validatePdfOptions({
        ...DEFAULT_PDF_OPTIONS,
        footer: "первая\nвторая",
      }),
    ).toContain("недопустимые");
  });

  it("читает и пишет кэш параметров", () => {
    const store = storage();
    expect(readCachedPdfOptions(store)).toEqual(DEFAULT_PDF_OPTIONS);

    const options = {
      ...DEFAULT_PDF_OPTIONS,
      paper: "letter" as const,
      footer: "стр. {page}",
    };
    writeCachedPdfOptions(options, store);

    expect(readCachedPdfOptions(store)).toEqual(options);

    store.setItem("liteoffice.docs.pdf.v1", "не json");
    expect(readCachedPdfOptions(store)).toEqual(DEFAULT_PDF_OPTIONS);
  });
});
