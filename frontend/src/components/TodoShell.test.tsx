import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import { TodoShell } from "./TodoShell";

describe("TodoShell flash messages", () => {
  afterEach(() => {
    vi.useRealTimers();
    delete window.showFlashMessage;
    document.body.innerHTML = "";
    document.body.style.overflow = "";
    document.body.className = "";
  });

  it("shows a dismissible alert without locking page scroll", () => {
    vi.useFakeTimers();

    const container = document.createElement("div");
    document.body.appendChild(container);

    const root = createRoot(container);

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

    const alert = document.querySelector(".todo-flash-stack .alert");
    expect(alert?.textContent).toContain("Saved");
    expect(document.body.style.overflow).toBe("");
    expect(document.body.classList.contains("modal-open")).toBe(false);
    expect(document.querySelector(".modal-backdrop")).toBeNull();

    act(() => {
      vi.advanceTimersByTime(4000);
    });

    expect(document.querySelector(".todo-flash-stack .alert")).toBeNull();

    act(() => {
      root.unmount();
    });
  });
});
