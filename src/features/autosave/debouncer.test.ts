import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createDebouncer } from "./debouncer";

describe("createDebouncer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("вызывает задачу после задержки", () => {
    const debouncer = createDebouncer(100);
    const task = vi.fn();

    debouncer.schedule("doc", task);
    vi.advanceTimersByTime(99);
    expect(task).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(task).toHaveBeenCalledTimes(1);
  });

  it("перезапускает таймер при повторном планировании", () => {
    const debouncer = createDebouncer(100);
    const first = vi.fn();
    const second = vi.fn();

    debouncer.schedule("doc", first);
    vi.advanceTimersByTime(50);
    debouncer.schedule("doc", second);
    vi.advanceTimersByTime(50);
    expect(first).not.toHaveBeenCalled();

    vi.advanceTimersByTime(50);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
  });

  it("отменяет задачу по ключу и полностью", () => {
    const debouncer = createDebouncer(100);
    const first = vi.fn();
    const second = vi.fn();

    debouncer.schedule("first", first);
    debouncer.cancel("first");
    vi.advanceTimersByTime(100);
    expect(first).not.toHaveBeenCalled();

    debouncer.schedule("first", first);
    debouncer.schedule("second", second);
    debouncer.cancelAll();
    vi.advanceTimersByTime(100);

    expect(first).not.toHaveBeenCalled();
    expect(second).not.toHaveBeenCalled();
  });
});
