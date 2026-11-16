import { describe, expect, it } from "vitest";
import { createEmptyDoc } from "../../lib/document";
import type { AutosaveRecord } from "./autosaveApi";
import { restoredTabs, tabFromAutosave } from "./session";

function record(overrides: Partial<AutosaveRecord> = {}): AutosaveRecord {
  return {
    docId: "doc-1",
    path: "/tmp/заметка.md",
    format: "md",
    fileName: "заметка.md",
    updatedAt: 100,
    document: createEmptyDoc(),
    ...overrides,
  };
}

describe("tabFromAutosave", () => {
  it("восстанавливает вкладку как изменённую", () => {
    const tab = tabFromAutosave(record());

    expect(tab).toMatchObject({
      id: "doc-1",
      path: "/tmp/заметка.md",
      format: "md",
      fileName: "заметка.md",
      dirty: true,
      showStart: false,
    });
    expect(tab.initialDocument).toEqual(createEmptyDoc());
  });

  it("подставляет имя для пустого названия", () => {
    const tab = tabFromAutosave(record({ fileName: "   ", path: null }));

    expect(tab.fileName).toBe("Без названия");
    expect(tab.path).toBeNull();
  });
});

describe("restoredTabs", () => {
  it("убирает повторяющиеся идентификаторы", () => {
    const tabs = restoredTabs([
      record({ docId: "a" }),
      record({ docId: "b" }),
      record({ docId: "a", fileName: "дубль.md" }),
    ]);

    expect(tabs.map((tab) => tab.id)).toEqual(["a", "b"]);
  });

  it("возвращает пустой список без записей", () => {
    expect(restoredTabs([])).toEqual([]);
  });
});
