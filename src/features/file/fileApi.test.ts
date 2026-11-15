import { describe, expect, it } from "vitest";
import { errorMessage } from "./fileApi";

describe("errorMessage", () => {
  it("разбирает структурированную ошибку Rust-ядра", () => {
    expect(
      errorMessage({ code: "io_error", message: "ошибка ввода-вывода: нет файла" }),
    ).toBe("ошибка ввода-вывода: нет файла");
  });

  it("принимает строку", () => {
    expect(errorMessage("что-то пошло не так")).toBe("что-то пошло не так");
  });

  it("возвращает запасное сообщение", () => {
    expect(errorMessage(null)).toBe("Не удалось выполнить операцию с файлом");
    expect(errorMessage({})).toBe("Не удалось выполнить операцию с файлом");
  });
});
