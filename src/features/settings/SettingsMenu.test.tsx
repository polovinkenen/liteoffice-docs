import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SettingsMenu from "./SettingsMenu";
import type { SettingsController } from "./useSettings";

function createController(
  overrides: Partial<SettingsController> = {},
): SettingsController {
  return {
    settings: { theme: "system", autosaveEnabled: true },
    resolvedTheme: "light",
    error: null,
    updateSettings: vi.fn(),
    ...overrides,
  };
}

describe("SettingsMenu", () => {
  it("открывает панель и меняет тему", () => {
    const updateSettings = vi.fn();
    render(<SettingsMenu controller={createController({ updateSettings })} />);

    fireEvent.click(screen.getByRole("button", { name: "Настройки" }));

    expect(
      screen.getByRole("dialog", { name: "Настройки" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("radio", { name: "Тёмная" }));
    expect(updateSettings).toHaveBeenCalledWith({ theme: "dark" });
  });

  it("переключает автосохранение", () => {
    const updateSettings = vi.fn();
    render(<SettingsMenu controller={createController({ updateSettings })} />);

    fireEvent.click(screen.getByRole("button", { name: "Настройки" }));
    fireEvent.click(
      screen.getByRole("switch", { name: "Автосохранение" }),
    );

    expect(updateSettings).toHaveBeenCalledWith({
      autosaveEnabled: false,
    });
  });

  it("показывает ошибку сохранения", () => {
    render(
      <SettingsMenu
        controller={createController({ error: "Не удалось сохранить" })}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Настройки" }));

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Не удалось сохранить",
    );
  });

  it("закрывается по Escape", () => {
    render(<SettingsMenu controller={createController()} />);

    fireEvent.click(screen.getByRole("button", { name: "Настройки" }));
    fireEvent.keyDown(document, { key: "Escape" });

    expect(
      screen.queryByRole("dialog", { name: "Настройки" }),
    ).not.toBeInTheDocument();
  });
});
