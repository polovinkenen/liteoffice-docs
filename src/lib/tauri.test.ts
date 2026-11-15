import { describe, expect, it } from "vitest";
import { isTauri } from "./tauri";

describe("isTauri", () => {
  it("возвращает false вне Tauri", () => {
    expect(isTauri()).toBe(false);
  });
});
