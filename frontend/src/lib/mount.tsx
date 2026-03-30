import type { ReactNode } from "react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../styles/foundation.css";

export function mountPage(elementId: string, page: ReactNode): void {
  const rootElement = document.getElementById(elementId);

  if (!rootElement) {
    throw new Error(`Missing React mount node: #${elementId}`);
  }

  const root = createRoot(rootElement);
  root.render(<StrictMode>{page}</StrictMode>);
}
