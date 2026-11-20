import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { OPEN_FILES_EVENT, takeOpenedPaths } from "./openedFiles";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("openedFiles", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("использует имя события Rust-ядра", () => {
    expect(OPEN_FILES_EVENT).toBe("app:open-files");
  });

  it("забирает накопленные пути командой take_opened_paths", async () => {
    vi.mocked(invoke).mockResolvedValue(["/tmp/док.md"]);

    const paths = await takeOpenedPaths();

    expect(invoke).toHaveBeenCalledWith("take_opened_paths");
    expect(paths).toEqual(["/tmp/док.md"]);
  });
});
