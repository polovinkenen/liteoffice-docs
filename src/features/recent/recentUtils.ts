export function recentLocation(path: string): string {
  const separator = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));

  return separator > 0 ? path.slice(0, separator) : "";
}

export function formatRecentDate(openedAt: number): string {
  if (!Number.isFinite(openedAt) || openedAt <= 0) {
    return "";
  }

  try {
    return new Intl.DateTimeFormat("ru-RU", {
      dateStyle: "short",
      timeStyle: "short",
    }).format(new Date(openedAt));
  } catch {
    return "";
  }
}
