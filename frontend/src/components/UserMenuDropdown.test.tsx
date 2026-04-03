import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { UserMenuDropdown } from "./UserMenuDropdown";

describe("UserMenuDropdown", () => {
  it("renders local items before fetched items and keeps logout last", () => {
    const markup = renderToStaticMarkup(
      <UserMenuDropdown
        currentUserEmail="user@example.com"
        localItems={[
          { name: "Домой", url: "https://auth.example.com" },
          { name: "Локальный пункт", url: "/local" },
        ]}
        fetchedItems={[
          { name: "Внешний пункт", url: "/remote" },
          { name: "Выйти", url: "/logout" },
        ]}
        logoutAction="/logout"
      />,
    );

    expect(markup.indexOf("Домой")).toBeLessThan(
      markup.indexOf("Локальный пункт"),
    );
    expect(markup.indexOf("Локальный пункт")).toBeLessThan(
      markup.indexOf("Внешний пункт"),
    );
    expect(markup.lastIndexOf("Выйти")).toBeGreaterThan(
      markup.indexOf("Внешний пункт"),
    );
  });
});
