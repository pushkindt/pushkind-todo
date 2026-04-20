import { ServiceNoAccessPage } from "@pushkind/frontend-shell/noAccess";

import { TodoShell } from "../components/TodoShell";
import { TodoShellFatalState } from "../components/TodoShellFatalState";
import {
  fetchHubMenuItems,
  fetchNoAccessData,
  fetchShellData,
} from "../lib/api";
import type { NoAccessData, ShellData, UserMenuItem } from "../lib/models";

export function NoAccessPage() {
  return (
    <ServiceNoAccessPage<NoAccessData, ShellData, UserMenuItem>
      serviceLabel="ToDo"
      fetchShellData={fetchShellData}
      fetchHubMenuItems={fetchHubMenuItems}
      fetchNoAccessData={fetchNoAccessData}
      ShellComponent={TodoShell}
      FatalStateComponent={TodoShellFatalState}
      menuLoadWarning="Failed to load auth navigation menu. Falling back to local ToDo menu only."
    />
  );
}
