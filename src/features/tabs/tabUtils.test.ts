import { describe, expect, it } from "vitest";
import {
  anyDirty,
  createBlankTab,
  findTabByPath,
  neighborAfterClose,
  nextUntitledNumber,
} from "./tabUtils";

describe("createBlankTab", () => {
  it("нумерует безымянные документы", () => {
    expect(createBlankTab(1).fileName).toBe("Без названия");
    expect(createBlankTab(2).fileName).toBe("Без названия 2");
  });

  it("создаёт чистую вкладку без пути", () => {
    const tab = createBlankTab(1);

    expect(tab.path).toBeNull();
    expect(tab.dirty).toBe(false);
    expect(tab.showStart).toBe(false);
    expect(tab.initialDocument.type).toBe("doc");
  });

  it("может сразу показывать стартовый экран", () => {
    expect(createBlankTab(1, true).showStart).toBe(true);
  });
});

describe("nextUntitledNumber", () => {
  it("учитывает занятые номера", () => {
    const tabs = [createBlankTab(1), createBlankTab(3)];

    expect(nextUntitledNumber(tabs)).toBe(4);
  });
});

describe("neighborAfterClose", () => {
  const tabs = [createBlankTab(1), createBlankTab(2), createBlankTab(3)];

  it("выбирает следующую вкладку", () => {
    expect(neighborAfterClose(tabs, tabs[1]!.id)).toBe(tabs[2]!.id);
  });

  it("выбирает предыдущую для последней вкладки", () => {
    expect(neighborAfterClose(tabs, tabs[2]!.id)).toBe(tabs[1]!.id);
  });

  it("возвращает null для единственной вкладки", () => {
    expect(neighborAfterClose([tabs[0]!], tabs[0]!.id)).toBeNull();
  });
});

describe("findTabByPath", () => {
  it("находит вкладку по пути", () => {
    const tab = { ...createBlankTab(1), path: "/tmp/документ.md" };
    const found = findTabByPath([tab], "/tmp/документ.md");

    expect(found?.id).toBe(tab.id);
    expect(findTabByPath([tab], "/tmp/другой.md")).toBeUndefined();
  });
});

describe("anyDirty", () => {
  it("проверяет наличие изменений", () => {
    expect(anyDirty([createBlankTab(1)])).toBe(false);
    expect(
      anyDirty([{ ...createBlankTab(1), dirty: true }]),
    ).toBe(true);
  });
});
