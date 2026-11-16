import { describe, expect, it } from "vitest";
import { formatRecentDate, recentLocation } from "./recentUtils";

describe("recentLocation", () => {
  it("возвращает родительский каталог", () => {
    expect(recentLocation("/Users/test/Документы/заметка.md")).toBe(
      "/Users/test/Документы",
    );
    expect(recentLocation("C:\\Users\\test\\заметка.md")).toBe(
      "C:\\Users\\test",
    );
  });

  it("возвращает пустую строку без каталога", () => {
    expect(recentLocation("заметка.md")).toBe("");
  });
});

describe("formatRecentDate", () => {
  it("форматирует дату", () => {
    expect(formatRecentDate(Date.UTC(2026, 10, 16, 12, 30))).not.toBe("");
  });

  it("возвращает пустую строку для невалидного значения", () => {
    expect(formatRecentDate(0)).toBe("");
    expect(formatRecentDate(Number.NaN)).toBe("");
  });
});
