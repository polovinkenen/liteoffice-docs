import { describe, expect, it } from "vitest";
import { normalizeLink } from "./formatting";

describe("normalizeLink", () => {
  it("добавляет https для домена", () => {
    expect(normalizeLink("example.com")).toBe("https://example.com");
  });

  it("сохраняет явные схемы и якоря", () => {
    expect(normalizeLink("http://example.com")).toBe("http://example.com");
    expect(normalizeLink("mailto:test@example.com")).toBe(
      "mailto:test@example.com",
    );
    expect(normalizeLink("#section")).toBe("#section");
  });

  it("возвращает null для пустого значения", () => {
    expect(normalizeLink("   ")).toBeNull();
  });
});
