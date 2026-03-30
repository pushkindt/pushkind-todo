import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="ToDo no-access scaffold"
    description="Этот документ нужен только для проверки Vite build pipeline. Живой маршрут /na ещё не переключён на React."
    routeLabel="GET /na"
  />,
);
