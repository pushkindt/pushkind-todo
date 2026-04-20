import type { ReactNode } from "react";
import { ServiceNavbar } from "@pushkind/frontend-shell/ServiceNavbar";

import type { NavigationItem, UserMenuItem } from "../lib/models";

type TodoNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
};

export function TodoNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
}: TodoNavbarProps) {
  return (
    <ServiceNavbar
      brand="ToDo"
      collapseId="todo-foundation-navbar"
      navigation={navigation}
      currentUserEmail={currentUserEmail}
      homeUrl={homeUrl}
      localMenuItems={localMenuItems}
      fetchedMenuItems={fetchedMenuItems}
      logoutAction="/logout"
      outerContainerClassName="container pt-2"
      search={search}
    />
  );
}
