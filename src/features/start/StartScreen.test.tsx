import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import StartScreen from "./StartScreen";

const recent = [
  {
    path: "/Users/test/Документы/заметка.md",
    fileName: "заметка.md",
    openedAt: Date.UTC(2026, 10, 16, 12, 30),
  },
  {
    path: "/Users/test/Документы/письмо.txt",
    fileName: "письмо.txt",
    openedAt: Date.UTC(2026, 10, 15, 9, 0),
  },
];

describe("StartScreen", () => {
  it("показывает действия и недавние документы", () => {
    render(
      <StartScreen
        recent={recent}
        onNew={vi.fn()}
        onOpen={vi.fn()}
        onOpenRecent={vi.fn()}
        onRemoveRecent={vi.fn()}
        onClearRecent={vi.fn()}
      />,
    );

    expect(screen.getByText("Добро пожаловать в LiteOffice Docs")).toBeInTheDocument();
    expect(screen.getByText("заметка.md")).toBeInTheDocument();
    expect(screen.getByText("письмо.txt")).toBeInTheDocument();
  });

  it("открывает и удаляет недавний документ", () => {
    const onOpenRecent = vi.fn();
    const onRemoveRecent = vi.fn();

    render(
      <StartScreen
        recent={recent}
        onNew={vi.fn()}
        onOpen={vi.fn()}
        onOpenRecent={onOpenRecent}
        onRemoveRecent={onRemoveRecent}
        onClearRecent={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByText("заметка.md"));
    expect(onOpenRecent).toHaveBeenCalledWith(recent[0]!.path);

    fireEvent.click(
      screen.getByRole("button", {
        name: "Убрать из недавних: заметка.md",
      }),
    );
    expect(onRemoveRecent).toHaveBeenCalledWith(recent[0]!.path);
  });

  it("показывает пустое состояние", () => {
    render(
      <StartScreen
        recent={[]}
        onNew={vi.fn()}
        onOpen={vi.fn()}
        onOpenRecent={vi.fn()}
        onRemoveRecent={vi.fn()}
        onClearRecent={vi.fn()}
      />,
    );

    expect(screen.getByText("Недавних документов нет.")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Очистить список" }),
    ).not.toBeInTheDocument();
  });

  it("вызывает создание и открытие документа", () => {
    const onNew = vi.fn();
    const onOpen = vi.fn();

    render(
      <StartScreen
        recent={[]}
        onNew={onNew}
        onOpen={onOpen}
        onOpenRecent={vi.fn()}
        onRemoveRecent={vi.fn()}
        onClearRecent={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Новый документ" }));
    fireEvent.click(screen.getByRole("button", { name: "Открыть…" }));

    expect(onNew).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenCalledTimes(1);
  });
});
