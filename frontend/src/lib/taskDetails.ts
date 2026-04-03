export function parseTaskIdFromPathname(pathname: string): number | undefined {
  const match = pathname.match(/^\/task\/(\d+)\/?$/);
  if (!match) {
    return undefined;
  }

  const taskId = Number.parseInt(match[1], 10);
  return Number.isFinite(taskId) && taskId > 0 ? taskId : undefined;
}

export function formatTaskDateTime(value?: string): string {
  if (!value) {
    return "—";
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return "—";
  }

  return trimmed.replace("T", " ").slice(0, 16);
}
