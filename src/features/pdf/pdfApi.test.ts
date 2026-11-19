import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { exportPdf, openPdfPreview } from "./pdfApi";
import { DEFAULT_PDF_OPTIONS } from "./model";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const document = {
  type: "doc",
  content: [{ type: "paragraph", content: [{ type: "text", text: "текст" }] }],
};

describe("pdfApi", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("передаёт документ, параметры и путь экспорта", async () => {
    vi.mocked(invoke).mockResolvedValue({
      path: "/tmp/док.pdf",
      pages: 2,
      bytes: 1024,
      warnings: [],
    });

    const result = await exportPdf(document, DEFAULT_PDF_OPTIONS, "/tmp/док.pdf", "Док");

    expect(invoke).toHaveBeenCalledWith("export_pdf", {
      document,
      options: DEFAULT_PDF_OPTIONS,
      outPath: "/tmp/док.pdf",
      title: "Док",
    });
    expect(result.pages).toBe(2);
  });

  it("запрашивает предпросмотр без пути", async () => {
    vi.mocked(invoke).mockResolvedValue({
      path: "/cache/док.pdf",
      pages: 1,
      bytes: 512,
      warnings: ["замечание"],
    });

    await exportPdf(document, DEFAULT_PDF_OPTIONS, null, null);

    expect(invoke).toHaveBeenCalledWith("export_pdf", {
      document,
      options: DEFAULT_PDF_OPTIONS,
      outPath: null,
      title: null,
    });
  });

  it("открывает предпросмотр по пути", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);

    await openPdfPreview("/cache/док.pdf");

    expect(invoke).toHaveBeenCalledWith("open_pdf_preview", {
      path: "/cache/док.pdf",
    });
  });
});
