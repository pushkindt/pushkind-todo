import { useEffect, useState } from "react";

import { fetchHubMenuItems, fetchShellData } from "./api";
import type { ShellData, UserMenuItem } from "./models";

type TodoShellState =
  | { status: "loading" }
  | {
      status: "ready";
      shell: ShellData;
      authMenuItems: UserMenuItem[];
      authMenuLoaded: boolean;
    }
  | { status: "error"; message: string };

export function useTodoShell(errorMessage: string) {
  const [state, setState] = useState<TodoShellState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    void fetchShellData()
      .then((shell) => {
        if (!active) {
          return;
        }

        setState({
          status: "ready",
          shell,
          authMenuItems: [],
          authMenuLoaded: false,
        });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setState({
          status: "error",
          message: error instanceof Error ? error.message : errorMessage,
        });
      });

    return () => {
      active = false;
    };
  }, [errorMessage]);

  useEffect(() => {
    if (state.status !== "ready" || state.authMenuLoaded) {
      return;
    }

    let active = true;

    void fetchHubMenuItems(state.shell.homeUrl, state.shell.currentUser.hubId)
      .then((authMenuItems) => {
        if (!active) {
          return;
        }

        setState((currentState) => {
          if (currentState.status !== "ready") {
            return currentState;
          }

          return {
            status: "ready",
            shell: currentState.shell,
            authMenuItems,
            authMenuLoaded: true,
          };
        });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        console.warn(
          "Failed to load auth navigation menu. Falling back to local ToDo menu only.",
          error,
        );

        setState((currentState) => {
          if (currentState.status !== "ready") {
            return currentState;
          }

          return {
            status: "ready",
            shell: currentState.shell,
            authMenuItems: currentState.authMenuItems,
            authMenuLoaded: true,
          };
        });
      });

    return () => {
      active = false;
    };
  }, [state]);

  return state;
}
