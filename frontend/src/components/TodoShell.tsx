import { ModalFlashShell } from "@pushkind/frontend-shell/ModalFlashShell";
import type { ReactNode } from "react";

import { TodoNavbar } from "./TodoNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type TodoShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
  children: ReactNode;
};

export function TodoShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
  children,
}: TodoShellProps) {
  return (
    <ModalFlashShell
      navbar={
        <TodoNavbar
          navigation={navigation}
          currentUserEmail={currentUserEmail}
          homeUrl={homeUrl}
          localMenuItems={localMenuItems}
          fetchedMenuItems={fetchedMenuItems}
          search={search}
        />
      }
      enablePopovers
    >
      {children}
    </ModalFlashShell>
  );
}
