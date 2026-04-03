type PhaseOneStatusCardProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOneStatusCard({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOneStatusCardProps) {
  return (
    <div className="card border-0 shadow-sm">
      <div className="card-body p-4 p-lg-5">
        <span className="badge text-bg-secondary mb-3">{badge}</span>
        <h1 className="h3 mb-3">{title}</h1>
        <p className="text-body-secondary mb-4">{description}</p>
        <dl className="row mb-0">
          <dt className="col-sm-4">Маршрут</dt>
          <dd className="col-sm-8">
            <code className="phase-one-code">{routeLabel}</code>
          </dd>
          <dt className="col-sm-4">Статус</dt>
          <dd className="col-sm-8">
            В Phase 1 этот экран существует только как проверка frontend build
            pipeline. Живая страница по-прежнему рендерится через Tera.
          </dd>
        </dl>
      </div>
    </div>
  );
}
