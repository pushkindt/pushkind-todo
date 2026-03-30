import { PhaseOneStatusCard } from "../components/PhaseOneStatusCard";

type PhaseOnePlaceholderPageProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOnePlaceholderPage({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOnePlaceholderPageProps) {
  return (
    <main className="phase-one-placeholder container py-4 py-lg-5">
      <div className="row justify-content-center">
        <div className="col-12 col-xl-8">
          <PhaseOneStatusCard
            badge={badge}
            title={title}
            description={description}
            routeLabel={routeLabel}
          />
        </div>
      </div>
    </main>
  );
}
