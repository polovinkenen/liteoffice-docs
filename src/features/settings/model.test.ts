import { describe, expect, it } from "vitest";
import {
  DEFAULT_SETTINGS,
  normalizeSettings,
  readCachedSettings,
  resolveTheme,
  SETTINGS_CACHE_KEY,
} from "./model";

describe("normalizeSettings", () => {
  it("возвращает значения по умолчанию для невалидных данных", () => {
    expect(normalizeSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(normalizeSettings("текст")).toEqual(DEFAULT_SETTINGS);
    expect(normalizeSettings({ theme: "blue" })).toMatchObject({
      theme: "system",
      autosaveEnabled: true,
    });
  });

  it("сохраняет валидные значения", () => {
    expect(
      normalizeSettings({ theme: "dark", autosaveEnabled: false }),
    ).toEqual({
      theme: "dark",
      autosaveEnabled: false,
    });
  });

  it("не теряет автосохранение при невалидной теме", () => {
    expect(
      normalizeSettings({ theme: "unknown", autosaveEnabled: false }),
    ).toEqual({
      theme: "system",
      autosaveEnabled: false,
    });
  });
});

describe("resolveTheme", () => {
  it("следует системной теме", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("использует явную тему", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });
});

describe("readCachedSettings", () => {
  it("читает настройки из кэша", () => {
    const storage = {
      getItem: (key: string) =>
        key === SETTINGS_CACHE_KEY
          ? JSON.stringify({ theme: "dark", autosaveEnabled: false })
          : null,
    } as unknown as Storage;

    expect(readCachedSettings(storage)).toEqual({
      theme: "dark",
      autosaveEnabled: false,
    });
  });

  it("возвращает значения по умолчанию при битом кэше", () => {
    const storage = {
      getItem: () => "не json",
    } as unknown as Storage;

    expect(readCachedSettings(storage)).toEqual(DEFAULT_SETTINGS);
  });
});
