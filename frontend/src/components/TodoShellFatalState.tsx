type TodoShellFatalStateProps = {
  message: string;
};

export function TodoShellFatalState({ message }: TodoShellFatalStateProps) {
  return (
    <main className="todo-foundation-shell">
      <section className="todo-foundation-card">
        <p className="todo-foundation-eyebrow">ToDo</p>
        <h1>Не удалось открыть React-оболочку ToDo</h1>
        <p className="mb-0 text-secondary">{message}</p>
      </section>
    </main>
  );
}
