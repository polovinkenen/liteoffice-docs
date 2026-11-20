import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { takeOpenedPaths } from "./openedFiles";
import { useOpenedFiles } from "./useOpenedFiles";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("./openedFiles", () => ({
  OPEN_FILES_EVENT: "app:open-files",
  takeOpenedPaths: vi.fn(),
}));

function setTauri(value: boolean) {
  if (value) {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true,
    });
  } else {
    delete (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  }
}

describe("useOpenedFiles", () => {
  let handler: EventCallback<string[]> | undefined;
  let stop: UnlistenFn;

  beforeEach(() => {
    handler = undefined;
    stop = vi.fn();
    setTauri(true);
    vi.mocked(takeOpenedPaths).mockReset();
    vi.mocked(listen)
      .mockReset()
      .mockImplementation((async (
        _event: string,
        callback: EventCallback<string[]>,
      ) => {
        handler = callback;
        return stop;
      }) as typeof listen);
  });

  afterEach(() => {
    setTauri(false);
  });

  it("открывает файлы, накопленные до загрузки фронтенда", async () => {
    vi.mocked(takeOpenedPaths).mockResolvedValue(["/tmp/док.md"]);
    const openPaths = vi.fn().mockResolvedValue(undefined);

    renderHook(() => useOpenedFiles(openPaths));

    await waitFor(() => {
      expect(openPaths).toHaveBeenCalledWith(["/tmp/док.md"]);
    });
    expect(listen).toHaveBeenCalledWith("app:open-files", expect.any(Function));
  });

  it("открывает файлы из события уже запущенного приложения", async () => {
    vi.mocked(takeOpenedPaths).mockResolvedValue([]);
    const openPaths = vi.fn().mockResolvedValue(undefined);

    renderHook(() => useOpenedFiles(openPaths));

    await waitFor(() => {
      expect(handler).toBeTypeOf("function");
    });
    expect(openPaths).not.toHaveBeenCalled();

    handler?.({ event: "app:open-files", id: 1, payload: ["/tmp/ещё.md"] });

    expect(openPaths).toHaveBeenCalledWith(["/tmp/ещё.md"]);
  });

  it("снимает подписку при размонтировании и не открывает пути", async () => {
    vi.mocked(takeOpenedPaths).mockResolvedValue([]);
    const openPaths = vi.fn().mockResolvedValue(undefined);

    const { unmount } = renderHook(() => useOpenedFiles(openPaths));

    await waitFor(() => {
      expect(handler).toBeTypeOf("function");
    });

    unmount();

    await waitFor(() => {
      expect(stop).toHaveBeenCalled();
    });
    expect(openPaths).not.toHaveBeenCalled();
  });
});
