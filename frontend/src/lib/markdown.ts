function escapeHtml(value: string) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

type MarkedWindow = Window & {
  marked?: {
    parse?: (value: string) => string;
  };
};

export function renderMarkdownToHtml(source: string) {
  const trimmedSource = source.trim();
  if (!trimmedSource) {
    return "";
  }

  if (typeof window === "undefined") {
    return escapeHtml(source).replaceAll("\n", "<br>");
  }

  const markedWindow = window as MarkedWindow;
  const parse = markedWindow.marked?.parse;
  if (typeof parse === "function") {
    return parse(source);
  }

  return escapeHtml(source).replaceAll("\n", "<br>");
}
