import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { DocumentWorkspace } from "../tabs/useDocumentWorkspace";
import { usePdfExport } from "./usePdfExport";
import { exportPdf, openPdfPreview } from "./pdfApi";
import { pickPdfPath } from "../file/dialogs";
import { DEFAULT_PDF_OPTIONS } from "./model";

vi.mock("./pdfApi", () => ({
  exportPdf: vi.fn(),
  openPdfPreview: vi.fn(),
}));

vi.mock("../file/dialogs", () => ({
  pickPdfPath: vi.fn(),
}));

function workspace(): DocumentWorkspace {
  return {
    activeEditor: { getJSON: () => ({ type: "doc", content: [] }) },
    activeTab: { path: null, fileName: "Новый.md" },
    reportWarnings: vi.fn(),
  } as unknown as DocumentWorkspace;
}

describe("usePdfExport", () => {
  beforeEach(() => {
    vi.mocked(exportPdf).mockReset();
    vi.mocked(openPdfPreview).mockReset();
    vi.mocked(pickPdfPath).mockReset();
  });

  it("открывает диалог и подтягивает сохранённые параметры", () => {
    const { result } = renderHook(() => usePdfExport(workspace()));

    act(() => result.current.show());

    expect(result.current.open).toBe(true);
    expect(result.current.initialOptions).toEqual(DEFAULT_PDF_OPTIONS);
  });

  it("делает предпросмотр через временный файл", async () => {
    vi.mocked(exportPdf).mockResolvedValue({
      path: "/cache/Новый.pdf",
      pages: 1,
      bytes: 700,
      warnings: ["замечание"],
    });
    vi.mocked(openPdfPreview).mockResolvedValue();
    const { result } = renderHook(() => usePdfExport(workspace()));

    await act(async () => {
      await result.current.preview(DEFAULT_PDF_OPTIONS);
    });

    expect(exportPdf).toHaveBeenCalledWith(
      { type: "doc", content: [] },
      DEFAULT_PDF_OPTIONS,
      null,
      "Новый",
    );
    expect(openPdfPreview).toHaveBeenCalledWith("/cache/Новый.pdf");
    expect(result.current.warnings).toEqual(["замечание"]);
    expect(result.current.result?.pages).toBe(1);
    expect(result.current.busy).toBe(false);
  });

  it("экспортирует в выбранный файл и передаёт предупреждения", async () => {
    vi.mocked(pickPdfPath).mockResolvedValue("/tmp/Новый.pdf");
    vi.mocked(exportPdf).mockResolvedValue({
      path: "/tmp/Новый.pdf",
      pages: 2,
      bytes: 900,
      warnings: ["Шрифт заменён"],
    });
    const target = workspace();
    const { result } = renderHook(() => usePdfExport(target));

    await act(async () => {
      await result.current.exportFile(DEFAULT_PDF_OPTIONS);
    });

    expect(pickPdfPath).toHaveBeenCalledWith(null, "Новый");
    expect(exportPdf).toHaveBeenCalledWith(
      expect.anything(),
      DEFAULT_PDF_OPTIONS,
      "/tmp/Новый.pdf",
      "Новый",
    );
    expect(target.reportWarnings).toHaveBeenCalledWith(["Шрифт заменён"]);
    expect(result.current.open).toBe(false);
  });

  it("показывает ошибку экспорта", async () => {
    vi.mocked(pickPdfPath).mockResolvedValue("/tmp/Новый.pdf");
    vi.mocked(exportPdf).mockRejectedValue({ message: "нет места" });
    const { result } = renderHook(() => usePdfExport(workspace()));

    await act(async () => {
      await result.current.exportFile(DEFAULT_PDF_OPTIONS);
    });

    expect(result.current.error).toBe("нет места");
    expect(result.current.open).toBe(false);
  });

  it("не открывает диалог без активного редактора", async () => {
    const empty = { ...workspace(), activeEditor: null } as DocumentWorkspace;
    const { result } = renderHook(() => usePdfExport(empty));

    act(() => result.current.show());

    expect(result.current.open).toBe(false);
    await waitFor(() => expect(exportPdf).not.toHaveBeenCalled());
  });
});
