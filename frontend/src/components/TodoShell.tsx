import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { TodoNavbar } from "./TodoNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

declare global {
  interface Window {
    bootstrap?: {
      Modal: {
        getOrCreateInstance: (
          element: string | Element,
          options?: object,
        ) => {
          hide: () => void;
          show: () => void;
        };
      };
      Popover?: new (element: Element) => { dispose?: () => void };
    };
    showFlashMessage?: (message: string, category?: string) => void;
  }
}

type TodoShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
  children: ReactNode;
};

export function TodoShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
  children,
}: TodoShellProps) {
  const flashTimeoutRef = useRef<number | null>(null);
  const [flashMessage, setFlashMessage] = useState<{
    category: string;
    message: string;
  } | null>(null);

  useEffect(() => {
    window.showFlashMessage = (message, category = "primary") => {
      if (flashTimeoutRef.current != null) {
        window.clearTimeout(flashTimeoutRef.current);
      }

      setFlashMessage({ message, category });
      flashTimeoutRef.current = window.setTimeout(() => {
        setFlashMessage(null);
        flashTimeoutRef.current = null;
      }, 4000);
    };

    const Bootstrap = window.bootstrap;
    const Popover = Bootstrap?.Popover;
    const popovers =
      Popover == null
        ? []
        : Array.from(
            document.querySelectorAll("[data-bs-toggle='popover']"),
          ).map((element) => new Popover(element as Element));

    return () => {
      delete window.showFlashMessage;
      if (flashTimeoutRef.current != null) {
        window.clearTimeout(flashTimeoutRef.current);
      }
      popovers.forEach((popover) => popover?.dispose?.());
    };
  }, []);

  return (
    <>
      <div
        id="flashMessages"
        className="todo-flash-stack position-fixed top-0 start-50 translate-middle-x p-3"
        aria-live="polite"
        aria-atomic="true"
      >
        {flashMessage ? (
          <div
            className={`alert alert-${flashMessage.category} alert-dismissible mb-0`}
            role="alert"
          >
            {flashMessage.message}
            <button
              type="button"
              className="btn-close"
              aria-label="Close"
              onClick={() => {
                if (flashTimeoutRef.current != null) {
                  window.clearTimeout(flashTimeoutRef.current);
                  flashTimeoutRef.current = null;
                }
                setFlashMessage(null);
              }}
            />
          </div>
        ) : null}
      </div>
      <TodoNavbar
        navigation={navigation}
        currentUserEmail={currentUserEmail}
        homeUrl={homeUrl}
        localMenuItems={localMenuItems}
        fetchedMenuItems={fetchedMenuItems}
        search={search}
      />
      {children}
    </>
  );
}
