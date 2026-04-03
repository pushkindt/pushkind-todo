import { useEffect, useState } from "react";

import { TodoShell } from "../components/TodoShell";
import { TodoShellFatalState } from "../components/TodoShellFatalState";
import { fetchNoAccessData } from "../lib/api";
import type { NoAccessData } from "../lib/models";
import { useTodoShell } from "../lib/useTodoShell";

type NoAccessState =
  | { status: "loading" }
  | { status: "ready"; data: NoAccessData }
  | { status: "error"; message: string };

export function NoAccessPage() {
  const shellState = useTodoShell("Не удалось загрузить оболочку ToDo.");
  const [noAccessState, setNoAccessState] = useState<NoAccessState>({
    status: "loading",
  });

  useEffect(() => {
    let active = true;

    void fetchNoAccessData()
      .then((data) => {
        if (!active) {
          return;
        }

        setNoAccessState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setNoAccessState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  if (shellState.status === "error") {
    return <TodoShellFatalState message={shellState.message} />;
  }

  if (shellState.status === "loading" || noAccessState.status === "loading") {
    return null;
  }

  if (noAccessState.status === "error") {
    return <TodoShellFatalState message={noAccessState.message} />;
  }

  return (
    <TodoShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <main className="container py-5 todo-shell-content">
        <div className="card shadow-sm">
          <div className="card-body p-4">
            <p className="text-uppercase text-secondary small mb-2">ToDo</p>
            <h1 className="h3 mb-3">Недостаточно прав для доступа к сервису</h1>
            <p className="text-secondary mb-3">
              Пользователь{" "}
              <strong>{noAccessState.data.currentUser.name}</strong> не имеет
              роли <code>{noAccessState.data.requiredRole}</code>.
            </p>
            <p className="text-secondary mb-4">
              Текущий email:{" "}
              <strong>{noAccessState.data.currentUser.email}</strong>
            </p>
            <div className="d-flex flex-column flex-sm-row gap-2">
              <a className="btn btn-primary" href={noAccessState.data.homeUrl}>
                Домой
              </a>
              <form method="POST" action="/logout">
                <button className="btn btn-outline-secondary" type="submit">
                  Выйти
                </button>
              </form>
            </div>
          </div>
        </div>
      </main>
    </TodoShell>
  );
}
