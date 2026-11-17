import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useDocumentWorkspace } from "./useDocumentWorkspace";

describe("useDocumentWorkspace", () => {
  it("не дублирует стартовую вкладку по плюсу", () => {
    const { result } = renderHook(() => useDocumentWorkspace());

    expect(result.current.tabs).toHaveLength(1);
    expect(result.current.activeTab.showStart).toBe(true);

    act(() => result.current.newStartTab());

    expect(result.current.tabs).toHaveLength(1);
  });

  it("открывает стартовый экран в новой вкладке", () => {
    const { result } = renderHook(() => useDocumentWorkspace());

    act(() => result.current.dismissStartScreen());
    expect(result.current.activeTab.showStart).toBe(false);

    act(() => result.current.newStartTab());

    expect(result.current.tabs).toHaveLength(2);
    expect(result.current.activeTab.showStart).toBe(true);
  });
});
