import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { PhaseOnePlaceholderPage } from "./PhaseOnePlaceholderPage";

describe("PhaseOnePlaceholderPage", () => {
  it("renders the provided route label", () => {
    const markup = renderToStaticMarkup(
      <PhaseOnePlaceholderPage
        badge="Phase 1"
        title="ToDo frontend scaffold"
        description="Build-only placeholder"
        routeLabel="GET /"
      />,
    );

    expect(markup).toContain("Phase 1");
    expect(markup).toContain("GET /");
    expect(markup).toContain("ToDo frontend scaffold");
  });
});
