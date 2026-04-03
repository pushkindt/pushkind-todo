import { useEffect, useRef } from "react";
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
  const ajaxFlashContentRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    window.showFlashMessage = (message, category = "primary") => {
      const flashes = ajaxFlashContentRef.current;
      const modal = window.bootstrap?.Modal.getOrCreateInstance(
        "#ajax-flash-modal",
        {},
      );

      if (!flashes || !modal) {
        return;
      }

      flashes.innerHTML = `<div class="alert alert-${category} alert-dismissible mb-0" role="alert">${message}<button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button></div>`;
      modal.show();
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
      popovers.forEach((popover) => popover?.dispose?.());
    };
  }, []);

  return (
    <>
      <div className="modal" tabIndex={-1} id="ajax-flash-modal">
        <div className="modal-dialog">
          <div className="modal-content">
            <div
              className="modal-body"
              id="ajax-flash-content"
              style={{ padding: 0 }}
              ref={ajaxFlashContentRef}
            />
          </div>
        </div>
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
