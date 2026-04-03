type TodoShellFatalStateProps = {
  message: string;
};

export function TodoShellFatalState({ message }: TodoShellFatalStateProps) {
  return (
    <main className="container py-5">
      <div className="alert alert-danger mb-0" role="alert">
        {message}
      </div>
    </main>
  );
}
