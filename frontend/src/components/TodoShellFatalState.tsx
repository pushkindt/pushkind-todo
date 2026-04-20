import { ShellFatalState } from "@pushkind/frontend-shell/ShellFatalState";

type TodoShellFatalStateProps = {
  message: string;
};

export function TodoShellFatalState({ message }: TodoShellFatalStateProps) {
  return (
    <ShellFatalState
      message={message}
      serviceLabel="ToDo"
      title="Не удалось открыть React-оболочку ToDo"
      shellClassName="todo-foundation-shell"
      cardClassName="todo-foundation-card"
      eyebrowClassName="todo-foundation-eyebrow"
    />
  );
}
