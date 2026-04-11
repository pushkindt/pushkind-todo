import { marked } from "marked";

export function renderMarkdownToHtml(source: string) {
  const trimmedSource = source.trim();
  if (!trimmedSource) {
    return "";
  }

  const rendered = marked.parse(source, { async: false });
  return typeof rendered === "string" ? rendered : source;
}
