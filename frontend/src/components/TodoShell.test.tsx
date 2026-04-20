import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TodoShell } from "./TodoShell";

describe("TodoShell flash messages", () => {
  afterEach(() => {
    delete window.bootstrap;
    delete window.showFlashMessage;
    document.body.innerHTML = "";
    document.body.style.overflow = "";
    document.body.className = "";
  });

  it("shows flash messages in the shared Bootstrap modal host", () => {
    const container = document.createElement("div");
    document.body.appendChild(container);

    const root = createRoot(container);
    const show = vi.fn();

    window.bootstrap = {
      Modal: {
        getOrCreateInstance: vi.fn(() => ({
          hide: vi.fn(),
          show,
        })),
      },
    };

    act(() => {
      root.render(
        <TodoShell
          navigation={[]}
          currentUserEmail="user@example.com"
          homeUrl="https://auth.example.com"
          localMenuItems={[]}
          fetchedMenuItems={[]}
        >
          <main>Content</main>
        </TodoShell>,
      );
    });

    act(() => {
      window.showFlashMessage?.("Saved", "primary");
    });

    const alert = document.querySelector("#ajax-flash-content .alert");
    expect(alert?.textContent).toContain("Saved");
    expect(show).toHaveBeenCalledTimes(1);
    expect(document.querySelector("#ajax-flash-modal")).not.toBeNull();

    act(() => {
      root.unmount();
    });
  });
});
