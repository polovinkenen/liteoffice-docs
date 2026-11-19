import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import PdfExportDialog from "./PdfExportDialog";
import { DEFAULT_PDF_OPTIONS } from "./model";

function renderDialog(overrides: Partial<Parameters<typeof PdfExportDialog>[0]> = {}) {
  const props = {
    initial: DEFAULT_PDF_OPTIONS,
    busy: false,
    error: null,
    warnings: [],
    result: null,
    onPreview: vi.fn(),
    onExport: vi.fn(),
    onCancel: vi.fn(),
    ...overrides,
  };

  render(<PdfExportDialog {...props} />);

  return props;
}

describe("PdfExportDialog", () => {
  it("показывает параметры страницы и экспортирует их", () => {
    const props = renderDialog();

    expect(
      screen.getByRole("dialog", { name: "Экспорт в PDF" }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Бумага")).toHaveValue("a4");
    expect(screen.getByLabelText("Ориентация")).toHaveValue("portrait");

    fireEvent.change(screen.getByLabelText("Бумага"), {
      target: { value: "letter" },
    });
    fireEvent.change(screen.getByLabelText("Ориентация"), {
      target: { value: "landscape" },
    });
    fireEvent.change(screen.getByLabelText("Верхний колонтитул"), {
      target: { value: "Отчёт" },
    });
    fireEvent.click(screen.getByLabelText("Нумерация страниц"));
    fireEvent.click(screen.getByRole("button", { name: "Экспорт…" }));

    expect(props.onExport).toHaveBeenCalledWith(
      expect.objectContaining({
        paper: "letter",
        orientation: "landscape",
        header: "Отчёт",
        pageNumbers: false,
      }),
    );
  });

  it("применяет пресет полей и правит отдельное поле", () => {
    const props = renderDialog();

    fireEvent.change(screen.getByLabelText("Пресет полей"), {
      target: { value: "narrow" },
    });
    const top = screen.getByLabelText("Сверху");

    expect(top).toHaveValue(12);

    fireEvent.change(top, { target: { value: "8" } });
    fireEvent.click(screen.getByRole("button", { name: "Предпросмотр" }));

    expect(props.onPreview).toHaveBeenCalledWith(
      expect.objectContaining({
        margins: { top: 8, right: 12, bottom: 12, left: 12 },
      }),
    );
    expect(screen.getByLabelText("Пресет полей")).toHaveValue("custom");
  });

  it("показывает ошибку, предупреждения и результат", () => {
    renderDialog({
      error: "слишком большие поля",
      warnings: ["Шрифт заменён"],
      result: { path: "/tmp/x.pdf", pages: 3, bytes: 2048, warnings: [] },
    });

    expect(screen.getByRole("alert")).toHaveTextContent("слишком большие поля");
    expect(screen.getByText("Шрифт заменён")).toBeInTheDocument();
    expect(screen.getByText(/Предпросмотр готов: 3 стр., 2 КБ/)).toBeInTheDocument();
  });

  it("закрывается по Escape и отмене", () => {
    const props = renderDialog();

    fireEvent.keyDown(document, { key: "Escape" });
    expect(props.onCancel).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Отмена" }));
    expect(props.onCancel).toHaveBeenCalledTimes(2);
  });

  it("блокирует действия во время формирования PDF", () => {
    renderDialog({ busy: true });

    expect(screen.getByRole("button", { name: "Формируем PDF…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Экспорт…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Отмена" })).toBeDisabled();
  });
});
