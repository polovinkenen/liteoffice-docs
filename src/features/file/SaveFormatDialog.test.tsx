import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SaveFormatDialog from "./SaveFormatDialog";

describe("SaveFormatDialog", () => {
  it("показывает форматы и подтверждает выбранный", () => {
    const onConfirm = vi.fn();
    render(
      <SaveFormatDialog
        current="md"
        onConfirm={onConfirm}
        onCancel={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("dialog", { name: "Сохранить как" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /Markdown/ })).toBeChecked();

    fireEvent.click(screen.getByRole("radio", { name: /DOCX/ }));
    fireEvent.click(screen.getByRole("button", { name: "Сохранить" }));

    expect(onConfirm).toHaveBeenCalledWith("docx");
  });

  it("показывает предупреждение для TXT", () => {
    render(
      <SaveFormatDialog
        current="md"
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("radio", { name: /Текст/ }));

    expect(
      screen.getByText(/TXT сохраняет только текст/),
    ).toBeInTheDocument();
  });

  it("закрывается по Escape и кнопке отмены", () => {
    const onCancel = vi.fn();
    const { rerender } = render(
      <SaveFormatDialog
        current="md"
        onConfirm={vi.fn()}
        onCancel={onCancel}
      />,
    );

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledTimes(1);

    rerender(
      <SaveFormatDialog
        current="md"
        onConfirm={vi.fn()}
        onCancel={onCancel}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Отмена" }));
    expect(onCancel).toHaveBeenCalledTimes(2);
  });
});
