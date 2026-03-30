import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="ToDo task page scaffold"
    description="Этот документ нужен только для проверки Vite build pipeline. Живой маршрут GET /task/{task_id} пока остаётся на Tera."
    routeLabel="GET /task/{task_id}"
  />,
);
