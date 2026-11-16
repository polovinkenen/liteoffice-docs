export interface Debouncer {
  schedule: (key: string, task: () => void) => void;
  cancel: (key: string) => void;
  cancelAll: () => void;
}

export function createDebouncer(delayMs: number): Debouncer {
  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  const cancel = (key: string) => {
    const timer = timers.get(key);

    if (timer !== undefined) {
      clearTimeout(timer);
      timers.delete(key);
    }
  };

  return {
    schedule(key, task) {
      cancel(key);
      const timer = setTimeout(() => {
        timers.delete(key);
        task();
      }, delayMs);
      timers.set(key, timer);
    },
    cancel,
    cancelAll() {
      for (const timer of timers.values()) {
        clearTimeout(timer);
      }
      timers.clear();
    },
  };
}
